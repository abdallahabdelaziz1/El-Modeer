pub mod mprocess;

use crate::metrics::mprocess::MProcess;
use crate::renderer::column::Column;
use crate::util::percent_of;

use heim::host;
use heim::units::frequency::megahertz;
use heim::units::time;
use std::collections::{HashMap, HashSet};
use std::time::{Duration, SystemTime};
use chrono::prelude::DateTime;
use std::time::{UNIX_EPOCH};
use chrono::Local;

use std::fs;
use std::path::{Path};
use sysinfo::{
    Component, ComponentExt, Disk, DiskExt, ProcessExt, ProcessStatus, ProcessorExt, System, SystemExt,
};
use users::{Users, UsersCache};

// #[derive(FromPrimitive, PartialEq, Copy, Clone)]
// pub enum Column { 
//     PID = 0,
//     PPID = 1,
//     User = 2,
//     Priority = 3,
//     Nice = 4,
//     Status = 5,
//     TTY = 6,
//     CPUPercentage = 7,
//     MemoryPercentage = 8,
//     Memory = 9,
//     VirtualMemory = 10,
//     CPUTime = 11,
//     StartTime = 12,
//     CMD = 13,
// }

#[derive(PartialEq, Eq)]
pub enum ProcessTableSortOrder {
    Ascending = 0,
    Descending = 1,
}

pub trait DiskFreeSpaceExt {
    fn get_perc_free_space(&self) -> f32;
}

impl DiskFreeSpaceExt for Disk {
    fn get_perc_free_space(&self) -> f32 {
        if self.get_total_space() < 1 {
            return 0.0;
        }
        percent_of(self.get_available_space(), self.get_total_space())
    }
}

pub struct NetworkInterface {
    pub name: String,
    pub ip: String,
    pub dest: String,
}

pub struct Sensor {
    pub name: String,
    pub current_temp: f32,
    pub critical: f32,
    pub high: f32,
}

impl From<&Component> for Sensor {
    fn from(c: &Component) -> Sensor {
        Sensor {
            name: c.get_label().to_owned(),
            current_temp: c.get_temperature(),
            critical: c.get_critical().unwrap_or(0.0),
            high: c.get_max(),
        }
    }
}

fn get_max_pid() -> u64 {
    if cfg!(target_os = "macos") {
        99999
    } else if cfg!(target_os = "linux") {
        match fs::read(&Path::new("/proc/sys/kernel/pid_max")) {
            Ok(data) => {
                let r = String::from_utf8_lossy(data.as_slice());
                r.trim().parse::<u64>().unwrap_or(32768)
            }
            Err(_) => 32768,
        }
    } else {
        32768
    }
}

fn get_max_pid_length() -> usize {
    format!("{:}", get_max_pid()).len()
}

#[derive(Default, Debug)]
pub struct ValAndPid<T> {
    pub val: T,
    pub pid: Option<i32>,
}
impl<T: PartialOrd> ValAndPid<T> {
    // fn update(&mut self, new: T, pid: i32) {
    //     if new > self.val {
    //         self.val = new;
    //         self.pid = Some(pid);
    //     }
    // }
}

#[derive(Default, Debug)]
pub struct Top {
    pub cum_cpu: ValAndPid<f64>,
    pub cpu: ValAndPid<f32>,
    pub mem: ValAndPid<u64>,
    pub virt: ValAndPid<u64>,
    pub read: ValAndPid<f64>,
    pub write: ValAndPid<f64>,
    pub iowait: ValAndPid<f64>,
}
impl Top {

}

pub struct CPUTimeApp {
  //  pub histogram_map: HistogramMap,
    pub cpu_utilization: u64,
    pub mem_utilization: u64,
    pub mem_total: u64,
    pub swap_utilization: u64,
    pub swap_total: u64,
    pub cpus: Vec<(String, f32)>,

    // Processes data 
    pub total_processes: usize,
    pub running_processes: u64,
    pub sleeping_processes: u64,
    pub stopped_processes: u64,
    pub zombie_processes: u64,
    pub processes: Vec<i32>,
    pub process_map: HashMap<i32, MProcess>,
    pub psortby: Column,
    pub psortorder: ProcessTableSortOrder,
    // pub disks: HashMap<String, ZDisk>,
    pub disk_write: u64,
    pub disk_read: u64,
    pub system: System,
    pub net_in: u64,
    pub net_out: u64,
    pub user_cache: UsersCache,
    pub cum_cpu_process: Option<MProcess>,
    pub top_pids: Top,
    pub frequency: u64,
    pub threads_total: usize,
    pub osname: String,
    pub release: String,
    pub version: String,
    pub arch: String,
    pub hostname: String,
    pub network_interfaces: Vec<NetworkInterface>,
    pub sensors: Vec<Sensor>,
    // pub gfx_devices: Vec<GraphicsDevice>,
    pub processor_name: String,
    pub started: chrono::DateTime<chrono::Local>,
    pub selected_process: Option<Box<MProcess>>,
    pub max_pid_len: usize,
    pub uptime: Duration,
    pub tick: Duration
}

impl CPUTimeApp {
    pub fn new(tick: Duration) -> CPUTimeApp { 
        let mut s = CPUTimeApp {
      //      histogram_map,
            cpus: vec![],
            system: System::new_all(),
            cpu_utilization: 0,
            mem_utilization: 0,
            mem_total: 0,
            swap_total: 0,
            swap_utilization: 0,

            total_processes: 0,
            running_processes: 0,
            sleeping_processes: 0,
            stopped_processes: 0,
            zombie_processes: 0,
          //  disks: HashMap::with_capacity(10),
            net_in: 0,
            net_out: 0,
            processes: Vec::with_capacity(400),
            process_map: HashMap::with_capacity(400),
            user_cache: UsersCache::new(),
            cum_cpu_process: None,
            frequency: 0,
            threads_total: 0,
            disk_read: 0,
            disk_write: 0,
            psortby: Column::CPUPercentage,
            psortorder: ProcessTableSortOrder::Descending,
            osname: String::from(""),
            release: String::from(""),
            version: String::from(""),
            arch: String::from(""),
            hostname: String::from(""),
            network_interfaces: vec![],
            sensors: vec![],
            processor_name: String::from(""),
            started: chrono::Local::now(),
            selected_process: None,
            max_pid_len: get_max_pid_length(),
            top_pids: Top::default(),
            uptime: Duration::from_secs(0),
            tick: tick,
        };
        s.system.refresh_all();
        s.system.refresh_all(); // apparently multiple refreshes are necessary to fill in all values.
        s
    }

    async fn get_platform(&mut self) {
        match host::platform().await {
            Ok(p) => {
                self.osname = p.system().to_owned();
                self.arch = p.architecture().as_str().to_owned();
                self.hostname = p.hostname().to_owned();
                self.version = p.version().to_owned();
                self.release = p.release().to_owned();
            }
            Err(_) => {
                self.osname = String::from("unknown");
                self.arch = String::from("unknown");
                self.hostname = String::from("unknown");
                self.version = String::from("unknown");
                self.release = String::from("unknown");
            }
        };
    }

    async fn get_uptime(&mut self) {
        if let Ok(u) = host::uptime().await {
            self.uptime = Duration::from_secs_f64(u.get::<time::second>());
        }
    }

    pub fn select_process(&mut self, highlighted_process: Option<Box<MProcess>>) {
        self.selected_process = highlighted_process;
    }

    fn update_process_list(&mut self, keep_order: bool) {
        let process_list = self.system.get_processes();
        #[cfg(target_os = "linux")]
      //  let client = &self.netlink_client;
        let mut current_pids: HashSet<i32> = HashSet::with_capacity(process_list.len());

        let mut top = Top::default();
        top.cum_cpu.val = match &self.cum_cpu_process {
            Some(p) => p.cum_cpu_usage,
            None => 0.0,
        };

        self.threads_total = 0;
        self.total_processes = process_list.len();
        self.running_processes = 0;
        self.sleeping_processes = 0;
        self.stopped_processes = 0;
        self.zombie_processes = 0;
        for (pid, process) in process_list {
            match process.status() {
            ProcessStatus::Run => self.running_processes += 1,
            ProcessStatus::Sleep => self.sleeping_processes += 1,
            ProcessStatus::Stop => self.stopped_processes += 1,
            ProcessStatus::Zombie => self.zombie_processes += 1,
            _ => (),
            }

            if let Some(zp) = self.process_map.get_mut(pid) {
                if zp.start_time == process.start_time() {
                    let disk_usage = process.disk_usage();
                    // check for PID reuse
                    zp.memory = process.memory();
                    zp.cpu_usage = process.cpu_usage();
                    zp.cum_cpu_usage += zp.cpu_usage as f64;
                    zp.status = process.status();
                    zp.priority = process.priority;
                    zp.nice = process.nice;
                    zp.virtual_memory = process.virtual_memory();
                    zp.threads_total = process.threads_total;
                    self.threads_total += zp.threads_total as usize;
                    zp.prev_read_bytes = zp.read_bytes;
                    zp.prev_write_bytes = zp.write_bytes;
                    zp.read_bytes = disk_usage.total_read_bytes;
                    zp.write_bytes = disk_usage.total_written_bytes;
                    zp.last_updated = SystemTime::now();

                    zp.et = match zp.end_time {
                        Some(t) => DateTime::<Local>::from(UNIX_EPOCH + Duration::from_secs(t)),
                        None => Local::now(),
                    };

                    // #[cfg(target_os = "linux")]
                    // zp.update_delay(client);

                    // top.update(zp, &self.histogram_map.tick);
                } else {
                    let user_name = self
                        .user_cache
                        .get_user_by_uid(process.uid)
                        .map(|user| user.name().to_string_lossy().to_string())
                        .unwrap_or(format!("{:}", process.uid));
                    let mprocess = MProcess::from_user_and_process(user_name, process);
                    self.threads_total += mprocess.threads_total as usize;

                    // top.update(zp, &self.histogram_map.tick);

                    self.process_map.insert(mprocess.pid, mprocess);
                }
            } else {
                let user_name = self
                    .user_cache
                    .get_user_by_uid(process.uid)
                    .map(|user| user.name().to_string_lossy().to_string())
                    .unwrap_or(format!("{:}", process.uid));
                #[allow(unused_mut)]
                let mut mprocess = MProcess::from_user_and_process(user_name, process);
                // #[cfg(target_os = "linux")]
                // mprocess.update_delay(client);

                self.threads_total += mprocess.threads_total as usize;

                // top.update(&mprocess, &self.histogram_map.tick);

                self.process_map.insert(mprocess.pid, mprocess);
            }
            current_pids.insert(*pid);
        }

        if keep_order {
            self.processes.retain(|pid| current_pids.contains(pid));
        } else {
            self.processes = current_pids.iter().cloned().collect();
        }

        // remove pids that are gone
        self.process_map.retain(|&k, _| current_pids.contains(&k));

        //set top cumulative process if we've changed it.
        if let Some(p) = top.cum_cpu.pid {
            if let Some(p) = self.process_map.get(&p) {
                self.cum_cpu_process = Some(p.clone())
            }
        } else if let Some(p) = &mut self.cum_cpu_process {
            if let Some(cp) = self.process_map.get(&p.pid) {
                if cp.start_time == p.start_time {
                    self.cum_cpu_process = Some(cp.clone());
                } else {
                    p.set_end_time();
                }
            } else {
                // our cumulative winner is dead
                p.set_end_time();
            }
        }

        self.top_pids = top;

        // update selected process
        if let Some(p) = self.selected_process.as_mut() {
            let pid = &p.pid;
            if let Some(proc) = self.process_map.get(pid) {
                self.selected_process = Some(Box::new(proc.clone()));
            } else {
                p.set_end_time();
            }
        }

        if !keep_order {
            self.sort_process_table();
        }
    }

    pub fn sort_process_table(&mut self) {
        let pm = &self.process_map;
        let sorter = MProcess::field_comparator(self.psortby);
        let sortorder = &self.psortorder;
        // let tick = self.histogram_map.tick;
        let tick = self.tick;
        self.processes.sort_by(|a, b| {
            let pa = pm.get(a).expect("Error in sorting the process table.");
            let pb = pm.get(b).expect("Error in sorting the process table.");

            let ord = sorter(pa, pb, &tick);
            match sortorder {
                ProcessTableSortOrder::Ascending => ord,
                ProcessTableSortOrder::Descending => ord.reverse(),
            }
        });
    }

    pub fn change_tick(&mut self, new_tick: Duration){
        self.tick = new_tick;
    }

    async fn update_frequency(&mut self) {
        let f = heim::cpu::frequency().await;
        if let Ok(f) = f {
            self.frequency = f.current().get::<megahertz>();
        }
    }

    // async fn update_disk(&mut self) {
    //     debug!("Updating Disks");

    //     static IGNORED_FILE_SYSTEMS: &[&[u8]] = &[
    //         b"sysfs",
    //         b"proc",
    //         b"tmpfs",
    //         b"cgroup",
    //         b"cgroup2",
    //         b"pstore",
    //         b"squashfs",
    //         b"iso9660",
    //     ];

    //     self.system.refresh_disks_list();
    //     let mut updated: HashMap<String, bool> = HashMap::with_capacity(self.disks.len());
    //     for k in self.disks.keys() {
    //         if k == "Total" {
    //             continue;
    //         }
    //         updated.insert(k.to_string(), false);
    //     }

    //     let mut total_available = 0;
    //     let mut total_space = 0;

    //     for d in self.system.get_disks().iter() {
    //         let name = d.get_name().to_string_lossy();
    //         let mp = d.get_mount_point().to_string_lossy();
    //         if cfg!(target_os = "linux") {
    //             let fs = d.get_file_system();
    //             if IGNORED_FILE_SYSTEMS.iter().any(|ignored| &fs == ignored) {
    //                 continue;
    //             }
    //             if mp.starts_with("/sys")
    //                 || mp.starts_with("/proc")
    //                 || mp.starts_with("/run")
    //                 || mp.starts_with("/dev")
    //                 || name.starts_with("shm")
    //                 || name.starts_with("sunrpc")
    //             {
    //                 continue;
    //             }
    //         }
    //         let name = get_device_name(d.get_name());
    //         let mut zd = self.disks.entry(name).or_insert(ZDisk::from_disk(&d));
    //         zd.size_bytes = d.get_total_space();
    //         zd.available_bytes = d.get_available_space();
    //         total_available += zd.available_bytes;
    //         total_space += zd.size_bytes;
    //         updated.insert(zd.name.to_string(), true);
    //         self.histogram_map.add_value_to(
    //             &HistogramKind::FileSystemUsedSpace(zd.name.to_string()),
    //             zd.get_used_bytes(),
    //         );
    //     }
    //     for (k, v) in updated.iter() {
    //         if !v {
    //             self.disks.remove(k);
    //         }
    //     }

    //     self.disk_read = self
    //         .process_map
    //         .iter()
    //         .map(|(_pid, p)| p.get_read_bytes_sec(&self.histogram_map.tick) as u64)
    //         .sum();
    //     self.disk_write = self
    //         .process_map
    //         .iter()
    //         .map(|(_pid, p)| p.get_write_bytes_sec(&self.histogram_map.tick) as u64)
    //         .sum();

    //     get_disk_io_metrics(&mut self.disks).await;

    //     let mut previous_io = IoMetrics {
    //         read_bytes: 0,
    //         write_bytes: 0,
    //     };
    //     let mut current_io = IoMetrics {
    //         read_bytes: 0,
    //         write_bytes: 0,
    //     };

    //     #[cfg(target_os = "linux")]
    //     for d in self.disks.values() {
    //         if d.mount_point.to_string_lossy() != "Total" {
    //             previous_io += d.previous_io;
    //             current_io += d.current_io;
    //         }
    //     }
    //     #[cfg(not(target_os = "linux"))]
    //     for p in self.process_map.values() {
    //         previous_io.read_bytes += p.prev_read_bytes;
    //         previous_io.write_bytes += p.prev_write_bytes;
    //         current_io.read_bytes += p.read_bytes;
    //         current_io.write_bytes += p.write_bytes;
    //     }

    //     self.update_disk_histograms(total_available, total_space, previous_io, current_io)
    //         .await;
    // }

    // pub async fn update_disk_histograms(
    //     &mut self,
    //     total_available: u64,
    //     total_space: u64,
    //     previous_io: IoMetrics,
    //     current_io: IoMetrics,
    // ) {
    //     let mut overall = self
    //         .disks
    //         .entry("Total".to_string())
    //         .or_insert(ZDisk::new_total());
    //     overall.available_bytes = total_available;
    //     overall.size_bytes = total_space;
    //     overall.previous_io = previous_io;
    //     overall.current_io = current_io;
    //     self.histogram_map.add_value_to(
    //         &HistogramKind::FileSystemUsedSpace(overall.name.to_string()),
    //         overall.get_used_bytes(),
    //     );

    //     for disk in self.disks.values() {
    //         self.histogram_map.add_value_to(
    //             &HistogramKind::IoRead(disk.name.to_string()),
    //             disk.get_read_bytes_sec(&self.histogram_map.tick) as u64,
    //         );
    //         self.histogram_map.add_value_to(
    //             &HistogramKind::IoWrite(disk.name.to_string()),
    //             disk.get_write_bytes_sec(&self.histogram_map.tick) as u64,
    //         );
    //     }
    // }

    pub async fn update_cpu(&mut self) {
        // debug!("Updating CPU");
        let procs = self.system.get_processors();
        let mut usage: f32 = 0.0;
        self.cpus.clear();
        let mut usagev: Vec<f32> = vec![];
        for (i, p) in procs.iter().enumerate() {
            if i == 0 {
                self.processor_name = p.get_name().to_owned();
            }
            let mut u = p.get_cpu_usage();
            if u.is_nan() {
                u = 0.0;
            }
            self.cpus.push((format!("{}", i + 1), u));
            usage += u;
            usagev.push(u);
        }
        if procs.is_empty() {
            self.cpu_utilization = 0;
        } else {
            usage /= procs.len() as f32;
            self.cpu_utilization = usage as u64;
        }
        // self.histogram_map
        //     .add_value_to(&HistogramKind::Cpu, self.cpu_utilization);
    }

    // pub async fn update_networks(&mut self) {
    //     let mut net_in = 0;
    //     let mut net_out = 0;
    //     for (_iface, data) in self.system.get_networks() {
    //         debug!("iface: {}", _iface);
    //         net_in += data.get_received();
    //         net_out += data.get_transmitted();
    //     }
    //     self.net_in = net_in;
    //     self.net_out = net_out;
    //     self.histogram_map
    //         .add_value_to(&HistogramKind::NetRx, self.net_in);
    //     self.histogram_map
    //         .add_value_to(&HistogramKind::NetTx, self.net_out);
    // }

    pub async fn update(&mut self, keep_order: bool) {
        // debug!("Updating Metrics");
        self.system.refresh_all();
        self.update_cpu().await;

        self.mem_utilization = self.system.get_used_memory();
        self.mem_total = self.system.get_total_memory();

        self.swap_utilization = self.system.get_used_swap();
        self.swap_total = self.system.get_total_swap();
        
        
        // let mem = percent_of(self.mem_utilization, self.mem_total) as u64;

        // self.histogram_map.add_value_to(&HistogramKind::Mem, mem);


        // self.update_networks().await;
        self.update_process_list(keep_order);
        self.update_frequency().await;
        // self.update_disk().await;
        self.get_platform().await;
        self.get_uptime().await;
        // self.update_gfx_devices();
        // self.update_gpu_utilization();
    }

    // pub async fn save_state(&mut self) {
    //     self.histogram_map.save_histograms();
    // }

    // pub fn writes_db_store(&self) -> bool {
    //     self.histogram_map.writes_db_store()
    // }
}
