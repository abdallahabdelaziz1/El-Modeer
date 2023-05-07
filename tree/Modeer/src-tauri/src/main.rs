
// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]


use std::path::Path;
use std::fs;
use std::io::prelude::*;
use std::fs::File;
use std::collections::hash_map::Entry::{Occupied, Vacant};
use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use procfs::process::Process;
use users::{Users, UsersCache};


#[derive(Clone,Debug,Serialize,Deserialize)]
struct ProcessRecord {
    name: String,
    pid: i32,
    ppid: i32,
    state: char,
    vmsize: u64,
    nice: i64,
    cpu_time: u64,
    username: String,
    rss: u64,
}

#[derive(Clone,Debug,Serialize,Deserialize)]
struct ProcessTreeNode {
    name: String,
    pid: i32,
    ppid: i32,
    state: char,
    vmsize: u64,
    nice: i64,
    cpu_time: u64,
    username: String,
    rss: u64,

    children: Vec<ProcessTreeNode>,
}

#[derive(Clone,Debug,Serialize,Deserialize)]
struct ProcessTree {
    root: ProcessTreeNode,
}

impl ProcessTreeNode {
    // constructor
    fn new(record : &ProcessRecord) -> ProcessTreeNode {
        ProcessTreeNode { 
            pid: (record.pid).clone(),
            name: (record.name).clone(),
            ppid: (record.ppid).clone(),
            state: (record.state).clone(),
            vmsize: (record.vmsize).clone(),
            nice: (record.nice).clone(),
            cpu_time: (record.cpu_time).clone(),
            username: (record.username).clone(),
            rss: (record.rss).clone(),

            children: Vec::new() 
        }
    }
}


// Given a status file path, return a hashmap with the following form:
// pid -> ProcessRecord
fn get_process_record(status_path: &Path) -> Option<ProcessRecord> {
    let mut pid : Option<i32> = None;
    let mut ppid : Option<i32> = None;
    let mut name : Option<String> = None;


    let mut reader = std::io::BufReader::new(File::open(status_path).unwrap());
    loop {
        let mut linebuf = String::new();
        match reader.read_line(&mut linebuf) {
            Ok(_) => {
                if linebuf.is_empty() {
                    break;
                }
                let parts : Vec<&str> = linebuf[..].splitn(2, ':').collect();
                if parts.len() == 2 {
                    let key = parts[0].trim();
                    let value = parts[1].trim();
                    match key {
                        "Name" => name = Some(value.to_string()),
                        "Pid" => pid = value.parse().ok(),
                        "PPid" => ppid = value.parse().ok(),
                        _ => (),
                    }
                }
            },
            Err(_) => break,
        }
    }

    if pid.is_none() || ppid.is_none() || name.is_none() {
        return None;
    }

    let cache = UsersCache::new();

    let proc = Process::new(pid.unwrap()).unwrap();
    let stat = proc.stat().unwrap();
    let user = cache.get_user_by_uid(proc.uid().unwrap()).unwrap();

    return Some(ProcessRecord { 
            pid: pid.unwrap(),
            name: name.unwrap(),
            ppid: ppid.unwrap(),
            state: stat.state,
            nice: stat.nice,
            cpu_time: (stat.utime + stat.stime)/procfs::ticks_per_second(),
            username: user.name().to_string_lossy().as_ref().to_string(),
            vmsize: stat.vsize,
            rss: stat.rss_bytes(),
        })
}


// build a simple struct (ProcessRecord) for each process
fn get_process_records() -> Vec<ProcessRecord> {
    let proc_directory = Path::new("/proc");

    // find potential process directories under /proc
    let proc_directory_contents = fs::read_dir(&proc_directory).unwrap();
    proc_directory_contents.filter_map(|entry| {
        let entry_path = entry.unwrap().path();
        if fs::metadata(entry_path.as_path()).unwrap().is_dir() {
            let status_path = entry_path.join("status");
            if let Ok(metadata) = fs::metadata(status_path.as_path()) {
                if metadata.is_file() {
                    return get_process_record(status_path.as_path());
                }
            }
        }
        None
    }).collect()
}

fn populate_node_helper(node: &mut ProcessTreeNode, pid_map: &HashMap<i32, &ProcessRecord>, ppid_map: &HashMap<i32, Vec<i32>>) {
    let pid = node.pid;
    let child_nodes = &mut node.children;
    match ppid_map.get(&pid) {
        Some(children) => {
            child_nodes.extend(children.iter().map(|child_pid| {
                let record = pid_map[child_pid];
                let mut child = ProcessTreeNode::new(record);
                populate_node_helper(&mut child, pid_map, ppid_map);
                child
            }));
        },
        None => {},
    }
}

fn populate_node(node : &mut ProcessTreeNode, records: &Vec<ProcessRecord>) {
    // key is a pid and its value is a vector of the whose parent pid is the key
    let mut ppid_map : HashMap<i32, Vec<i32>> = HashMap::new();
    let mut pid_map : HashMap<i32, &ProcessRecord> = HashMap::new();
    for record in records.iter() {
        pid_map.insert(record.pid, record);
        match ppid_map.entry(record.ppid) {
            Vacant(entry) => { entry.insert(vec![record.pid]); },
            Occupied(mut entry) => { entry.get_mut().push(record.pid); },
        };
    }

    populate_node_helper(node, &pid_map, &ppid_map);
}

fn build_process_tree() -> ProcessTree {
    let records = get_process_records();
    let mut tree = ProcessTree {
        root : ProcessTreeNode::new(
            &ProcessRecord {
                name: "".to_string(),
                pid: 0,
                ppid: -1,
                state: ' ',
                vmsize: 0,
                nice: 0,
                cpu_time: 0,
                username: "".to_string(),
                rss: 0,
            })
    };

    {
        let root = &mut tree.root;
        populate_node(root, &records);
    }
    tree
}

#[tauri::command]
fn get_processes() -> String {
    let ptree = build_process_tree();

    // Serialize it to a JSON string.
    let json_str = serde_json::to_string(&ptree.root).unwrap();
    let result = json_str.replace(",\"children\":[]", "");
    return result;
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![get_processes])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
