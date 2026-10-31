//! Speech runs off the simulation thread, with two queued requests and cancellation.
use super::*;
use std::{
    process::{Command, Stdio},
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
        mpsc,
    },
};
#[derive(Resource)]
pub struct Speech {
    sender: mpsc::SyncSender<(u64, String)>,
    latest: Arc<AtomicU64>,
    submitted: u64,
}
impl Default for Speech {
    fn default() -> Self {
        let (sender, receiver) = mpsc::sync_channel::<(u64, String)>(2);
        let latest = Arc::new(AtomicU64::new(0));
        let check = latest.clone();
        std::thread::spawn(move || {
            let dir = std::env::temp_dir().join(format!("nord-radio-{}", std::process::id()));
            let _ = std::fs::create_dir_all(&dir);
            let mut cache = std::collections::VecDeque::<(String, std::path::PathBuf)>::new();
            let mut file_id = 0u64;
            while let Ok((mut serial, mut text)) = receiver.recv() {
                while let Ok(next) = receiver.try_recv() {
                    (serial, text) = next;
                }
                if check.load(Ordering::Relaxed) != serial {
                    continue;
                }
                let path = if let Some((_, p)) = cache.iter().find(|(t, _)| *t == text) {
                    p.clone()
                } else {
                    file_id += 1;
                    let p = dir.join(format!("{file_id}.wav"));
                    let result = Command::new("/usr/bin/say")
                        .args([
                            "-r",
                            "175",
                            "--file-format=WAVE",
                            "--data-format=LEI16@22050",
                            "-o",
                        ])
                        .arg(&p)
                        .arg(&text)
                        .stdout(Stdio::null())
                        .stderr(Stdio::null())
                        .status();
                    if !result.is_ok_and(|s| s.success()) {
                        continue;
                    }
                    filter(&p);
                    cache.push_back((text, p.clone()));
                    if cache.len() > 16 {
                        if let Some((_, old)) = cache.pop_front() {
                            let _ = std::fs::remove_file(old);
                        }
                    }
                    p
                };
                if check.load(Ordering::Relaxed) != serial {
                    continue;
                }
                if let Ok(mut child) = Command::new("/usr/bin/afplay")
                    .arg(path)
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .spawn()
                {
                    loop {
                        if check.load(Ordering::Relaxed) != serial {
                            let _ = child.kill();
                            let _ = child.wait();
                            break;
                        }
                        match child.try_wait() {
                            Ok(Some(_)) | Err(_) => break,
                            _ => std::thread::sleep(std::time::Duration::from_millis(20)),
                        }
                    }
                }
            }
            let _ = std::fs::remove_dir_all(dir);
        });
        Self {
            sender,
            latest,
            submitted: 0,
        }
    }
}
// Gentle telephone-band coloration; preserve WAV headers and bound sample amplitude.
fn filter(path: &std::path::Path) {
    let Ok(mut data) = std::fs::read(path) else {
        return;
    };
    if data.len() < 44 || &data[0..4] != b"RIFF" {
        return;
    }
    let mut at = 12;
    let (mut low, mut dc) = (0f64, 0f64);
    while at + 8 <= data.len() {
        let n = u32::from_le_bytes(data[at + 4..at + 8].try_into().unwrap()) as usize;
        let end = (at + 8 + n).min(data.len());
        if &data[at..at + 4] == b"data" {
            for chunk in data[at + 8..end].chunks_exact_mut(2) {
                let x = i16::from_le_bytes([chunk[0], chunk[1]]) as f64;
                dc += 0.08 * (x - dc);
                low += 0.55 * ((x - dc) - low);
                chunk.copy_from_slice(&(low.clamp(-30000., 30000.) as i16).to_le_bytes());
            }
            break;
        }
        at = end + (n % 2);
    }
    let _ = std::fs::write(path, data);
}
impl Speech {
    pub fn update(&mut self, generation: u64, naval: Option<&State>) {
        let serial = naval.map_or(0, |n| (generation.wrapping_add(1) << 32) ^ n.report_serial);
        if serial == self.submitted {
            return;
        }
        self.latest.store(serial, Ordering::Relaxed);
        if let Some(n) = naval.filter(|n| n.report_serial > 0) {
            let text = n
                .report
                .replace("/", ", ")
                .replace("NM", "nautical miles")
                .replace("FT", "feet");
            if self.sender.try_send((serial, text)).is_err() {
                return;
            }
        }
        self.submitted = serial;
    }
}
#[cfg(test)]
impl Speech {
    pub fn silent() -> Self {
        let (sender, _) = mpsc::sync_channel(1);
        Self {
            sender,
            latest: Arc::new(AtomicU64::new(0)),
            submitted: 0,
        }
    }
}
