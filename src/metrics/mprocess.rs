use crate::renderer::column::Column;
use heim::process;
use heim::process::ProcessError;
use libc::getpriority;
use libc::{id_t, setpriority};
use std::cmp::Ordering::{self, Equal};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use std::path::{PathBuf};
use std::fs::read_link;
use sysinfo::Process;
use sysinfo::ProcessExt;
use sysinfo::ProcessStatus;

use chrono::prelude::DateTime;
use chrono::Duration as CDuration;
use chrono::Local;
use crate::{convert_result_to_string, convert_error_to_string};

pub fn get_tty(process: &sysinfo::Process) -> String {
    let pid = process.pid();
    let tty_path = format!("/proc/{}/fd/0", pid);
    let path = PathBuf::from(&tty_path);

    if let Ok(target) = read_link(&path) {
        if let Some(tty_os_str) = target.file_name() {
            let tty_string = tty_os_str.to_string_lossy().into_owned();
            if tty_string.chars().all(|c| c.is_digit(10)) {
                return "pts/".to_string() + tty_string.as_str();
            } else {
                return "pts/0".to_string();
            }
        }
    }

    "?".to_string()
}

#[derive(Clone)]
pub struct MProcess {
    pub pid: i32,
    pub ppid: i32,
    pub uid: u32,
    pub user_name: String,
    pub tty: String,
    pub memory: u64,
    pub cpu_usage: f32,
    pub cum_cpu_usage: f64,
    pub command: Vec<String>,
    pub exe: String,
    pub status: ProcessStatus,
    pub name: String,
    pub priority: i32,
    pub nice: i32,
    pub virtual_memory: u64,
    pub threads_total: u64,
    pub read_bytes: u64,
    pub write_bytes: u64,
    pub prev_read_bytes: u64,
    pub prev_write_bytes: u64,
    pub last_updated: SystemTime,
    pub end_time: Option<u64>,
    pub start_time: u64,
    pub cpu_time: u64,
    pub gpu_usage: u64,
    pub fb_utilization: u64,
    pub enc_utilization: u64,
    pub dec_utilization: u64,
    pub sm_utilization: u64,
    pub io_delay: Duration,
    pub swap_delay: Duration,
    pub prev_io_delay: Duration,
    pub prev_swap_delay: Duration,
    pub et: DateTime<Local>, 
}

impl MProcess {
    pub fn from_user_and_process(user_name: String, process: &Process) -> Self {
        let disk_usage = process.disk_usage();
        MProcess {
            uid: process.uid,
            user_name,
            pid: process.pid(),
            ppid: process.parent().unwrap_or_else(|| 1), // if you can't get the parent, it's init
            tty: get_tty(process), 
            memory: process.memory(),
            cpu_usage: process.cpu_usage(),
            command: process.cmd().to_vec(),
            status: process.status(),
            exe: format!("{}", process.exe().display()),
            name: process.name().to_string(),
            cum_cpu_usage: process.cpu_usage() as f64,
            priority: process.priority,
            nice: process.nice,
            virtual_memory: process.virtual_memory(),
            threads_total: process.threads_total,
            read_bytes: disk_usage.total_read_bytes,
            write_bytes: disk_usage.total_written_bytes,
            prev_read_bytes: disk_usage.total_read_bytes,
            prev_write_bytes: disk_usage.total_written_bytes,
            last_updated: SystemTime::now(),
            end_time: None,
            start_time: process.start_time(),
            cpu_time: process.cpu_time(), // TODO: check the number again 
            gpu_usage: 0,
            fb_utilization: 0,
            enc_utilization: 0,
            dec_utilization: 0,
            sm_utilization: 0,
            io_delay: Duration::from_nanos(0),
            swap_delay: Duration::from_nanos(0),
            prev_io_delay: Duration::from_nanos(0),
            prev_swap_delay: Duration::from_nanos(0),
            et: Local::now(),
        }
    }
    pub fn get_read_bytes_sec(&self, tick_rate: &Duration) -> f64 {
        (self.read_bytes - self.prev_read_bytes) as f64 / tick_rate.as_secs_f64()
    }
    pub fn get_write_bytes_sec(&self, tick_rate: &Duration) -> f64 {
        (self.write_bytes - self.prev_write_bytes) as f64 / tick_rate.as_secs_f64()
    }    
    
    pub async fn suspend(&self) -> String {
        match process::get(self.pid).await {
            Ok(p) => convert_result_to_string!(p.suspend().await),
            Err(e) => convert_error_to_string!(e),
        }
    }

    pub async fn resume(&self) -> String {
        match process::get(self.pid).await {
            Ok(p) => convert_result_to_string!(p.resume().await),
            Err(e) => convert_error_to_string!(e),
        }
    }

    pub async fn kill(&self) -> String {
        match process::get(self.pid).await {
            Ok(p) => convert_result_to_string!(p.kill().await),
            Err(e) => convert_error_to_string!(e),
        }
    }

    pub async fn terminate(&self) -> String {
        match process::get(self.pid).await {
            Ok(p) => convert_result_to_string!(p.terminate().await),
            Err(e) => convert_error_to_string!(e),
        }
    }

    pub fn nice(&mut self) -> String {
        self.set_priority(19)
    }

    pub fn get_run_duration(&self) -> CDuration {
        let start_time = DateTime::<Local>::from(UNIX_EPOCH + Duration::from_secs(self.start_time));
        self.et - start_time
    }

    pub fn get_io_wait(&self, tick_rate: &Duration) -> f64 {
        ((self.io_delay.as_secs_f64() - self.prev_io_delay.as_secs_f64()) / tick_rate.as_secs_f64())
            * 100.0
    }

    pub fn get_total_io_wait(&self) -> f64 {
        let process_duration = self
            .get_run_duration()
            .to_std()
            .expect("Duration out of expected range!");
        (self.io_delay.as_secs_f64() / process_duration.as_secs_f64()) * 100.0
    }

    pub fn get_swap_wait(&self, tick_rate: &Duration) -> f64 {
        ((self.swap_delay.as_secs_f64() - self.prev_swap_delay.as_secs_f64())
            / tick_rate.as_secs_f64())
            * 100.0
    }

    pub fn get_total_swap_wait(&self) -> f64 {
        let process_duration = self
            .get_run_duration()
            .to_std()
            .expect("Duration out of expected range!");
        (self.swap_delay.as_secs_f64() / process_duration.as_secs_f64()) * 100.0
    }

    pub fn set_priority(&mut self, priority: i32) -> String {
        let mut result = unsafe { setpriority(0, self.pid as id_t, priority) };

        if result < 0 {
            String::from("Couldn't set priority.")
        } else {
            unsafe {
                result = getpriority(0, self.pid as id_t);
            }
            self.priority = result + 20;
            self.nice = result;
            String::from("Priority Set.")
        }
    }

    pub fn set_end_time(&mut self) {
        if self.end_time.is_none() {
            self.end_time = match SystemTime::now().duration_since(UNIX_EPOCH) {
                Ok(t) => Some(t.as_secs()),
                Err(_) => panic!("System time before unix epoch??"),
            };
        }
    }

    /// returns a pointer to a comparator function, not a closure
    pub fn field_comparator(
        sortfield: Column,
    ) -> fn(&Self, &Self, &Duration) -> Ordering {
        match sortfield {
            Column::CPUPercentage => {
                |pa, pb, _tick| pa.cpu_usage.partial_cmp(&pb.cpu_usage).unwrap_or(Equal)
            }
            Column::Memory => |pa, pb, _tick| pa.memory.cmp(&pb.memory),
            Column::MemoryPercentage => |pa, pb, _tick| pa.memory.cmp(&pb.memory),
            Column::User => |pa, pb, _tick| pa.user_name.cmp(&pb.user_name),
            Column::TTY => |pa, pb, _tick| pa.tty.partial_cmp(&pb.tty).unwrap_or(Equal),
            Column::PID => |pa, pb, _tick| pa.pid.cmp(&pb.pid),
            Column::PPID => |pa, pb, _tick| pa.ppid.cmp(&pb.ppid),
            Column::Status => {
                |pa, pb, _tick| pa.status.to_single_char().cmp(pb.status.to_single_char())
            }
            Column::Priority => |pa, pb, _tick| pa.priority.cmp(&pb.priority),
            Column::Nice => |pa, pb, _tick| pa.nice.cmp(&pb.nice),
            Column::VirtualMemory => |pa, pb, _tick| pa.virtual_memory.cmp(&pb.virtual_memory),
            Column::CPUTime => |pa, pb, _tick| pa.cpu_time.cmp(&pb.cpu_time),
            Column::StartTime => |pa, pb, _tick| pa.start_time.cmp(&pb.start_time),
            Column::CMD => |pa, pb, _tick| pa.name.cmp(&pb.name),
        }
    }
}

pub trait ProcessStatusExt {
    fn to_single_char(&self) -> &str;
}

impl ProcessStatusExt for ProcessStatus {
    fn to_single_char(&self) -> &str {
        match *self {
            ProcessStatus::Idle => "I",
            ProcessStatus::Run => "R",
            ProcessStatus::Sleep => "S",
            ProcessStatus::Stop => "T",
            ProcessStatus::Zombie => "Z",
            ProcessStatus::Tracing => "t",
            ProcessStatus::Dead => "x",
            ProcessStatus::Wakekill => "K",
            ProcessStatus::Waking => "W",
            ProcessStatus::Parked => "P",
            ProcessStatus::Unknown(_) => "U",
        }
    }
}
