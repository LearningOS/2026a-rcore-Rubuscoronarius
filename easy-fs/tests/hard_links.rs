use easy_fs::{BlockDevice, EasyFileSystem, BLOCK_SZ};
use std::sync::{Arc, Mutex};

struct MemoryDisk(Mutex<Vec<[u8; BLOCK_SZ]>>);
impl BlockDevice for MemoryDisk {
    fn read_block(&self, id: usize, buf: &mut [u8]) {
        buf.copy_from_slice(&self.0.lock().unwrap()[id]);
    }
    fn write_block(&self, id: usize, buf: &[u8]) {
        self.0.lock().unwrap()[id].copy_from_slice(buf);
    }
}

#[test]
fn links_survive_unlink_and_reclaim_inode_and_blocks() {
    let disk = Arc::new(MemoryDisk(Mutex::new(vec![[0; BLOCK_SZ]; 4096])));
    let fs = EasyFileSystem::create(disk, 4096, 1);
    let root = EasyFileSystem::root_inode(&fs);
    let file = root.create("original").unwrap();
    let id = file.stat().0;
    let data = vec![42; BLOCK_SZ * 30];
    assert_eq!(file.write_at(0, &data), data.len());
    assert!(root.link("original", "alias"));
    assert!(!root.link("original", "alias"));
    let alias = root.find("alias").unwrap();
    assert_eq!(alias.stat(), (id, 2));
    assert!(Arc::ptr_eq(&file, &alias));
    drop(root.unlink("original").unwrap());
    assert!(root.find("original").is_none());
    assert_eq!(alias.stat(), (id, 1));
    drop(root.unlink("alias").unwrap());
    assert_eq!(file.stat(), (id, 0));
    let mut read = vec![0; data.len()];
    assert_eq!(file.read_at(0, &mut read), data.len());
    assert_eq!(read, data);
    drop(alias);
    file.reclaim();
    drop(file);
    let replacement = root.create("replacement").unwrap();
    assert_eq!(replacement.stat().0, id);
    assert_eq!(replacement.write_at(0, &data), data.len());
    assert!(root.find("").is_none());
    assert_eq!(root.ls(), vec!["replacement"]);
}
