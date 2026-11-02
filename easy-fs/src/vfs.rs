use super::{
    block_cache_sync_all, get_block_cache, BlockDevice, DirEntry, DiskInode, DiskInodeType,
    EasyFileSystem, DIRENT_SZ,
};
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
use spin::{Mutex, MutexGuard};
/// Virtual filesystem layer over easy-fs
pub struct Inode {
    block_id: usize,
    block_offset: usize,
    fs: Arc<Mutex<EasyFileSystem>>,
    block_device: Arc<dyn BlockDevice>,
}

impl Inode {
    /// Create a vfs inode
    pub fn new(
        block_id: u32,
        block_offset: usize,
        fs: Arc<Mutex<EasyFileSystem>>,
        block_device: Arc<dyn BlockDevice>,
    ) -> Self {
        Self {
            block_id: block_id as usize,
            block_offset,
            fs,
            block_device,
        }
    }
    /// Call a function over a disk inode to read it
    fn read_disk_inode<V>(&self, f: impl FnOnce(&DiskInode) -> V) -> V {
        get_block_cache(self.block_id, Arc::clone(&self.block_device))
            .lock()
            .read(self.block_offset, f)
    }
    /// Call a function over a disk inode to modify it
    fn modify_disk_inode<V>(&self, f: impl FnOnce(&mut DiskInode) -> V) -> V {
        get_block_cache(self.block_id, Arc::clone(&self.block_device))
            .lock()
            .modify(self.block_offset, f)
    }
    /// Find inode under a disk inode by name
    fn find_inode_id(&self, name: &str, disk_inode: &DiskInode) -> Option<u32> {
        // assert it is a directory
        assert!(disk_inode.is_dir());
        let file_count = (disk_inode.size as usize) / DIRENT_SZ;
        let mut dirent = DirEntry::empty();
        for i in 0..file_count {
            assert_eq!(
                disk_inode.read_at(DIRENT_SZ * i, dirent.as_bytes_mut(), &self.block_device,),
                DIRENT_SZ,
            );
            if dirent.name() == name {
                return Some(dirent.inode_id() as u32);
            }
        }
        None
    }
    /// Find inode under current inode by name
    pub fn find(&self, name: &str) -> Option<Arc<Inode>> {
        if name.is_empty() {
            return None;
        }
        let fs = self.fs.lock();
        let inode_id = self.read_disk_inode(|disk_inode| self.find_inode_id(name, disk_inode))?;
        drop(fs);
        Some(self.cached_inode(inode_id))
    }
    fn cached_inode(&self, inode_id: u32) -> Arc<Inode> {
        let mut fs = self.fs.lock();
        if let Some(inode) = fs.open_inodes.get(&inode_id).and_then(|w| w.upgrade()) {
            return inode;
        }
        let (block, offset) = fs.get_disk_inode_pos(inode_id);
        let inode = Arc::new(Self::new(
            block,
            offset,
            self.fs.clone(),
            self.block_device.clone(),
        ));
        fs.open_inodes.insert(inode_id, Arc::downgrade(&inode));
        inode
    }

    /// Return the inode number and current link count.
    pub fn stat(&self) -> (u32, u32) {
        let fs = self.fs.lock();
        let id = fs.inode_id(self.block_id as u32, self.block_offset);
        let links = self.read_disk_inode(|inode| inode.nlink);
        (id, links)
    }

    /// Reclaim an unlinked inode after its last open handle is gone.
    pub fn reclaim(&self) {
        let mut fs = self.fs.lock();
        let id = fs.inode_id(self.block_id as u32, self.block_offset);
        self.modify_disk_inode(|inode| {
            if inode.nlink == 0 {
                for block in inode.clear_size(&self.block_device) {
                    fs.dealloc_data(block);
                }
                fs.dealloc_inode(id);
            }
        });
        block_cache_sync_all();
    }

    /// Add a directory entry pointing to an existing regular file.
    pub fn link(&self, old: &str, new: &str) -> bool {
        if old == new || new.is_empty() || new.len() > 27 {
            return false;
        }
        let mut fs = self.fs.lock();
        let id = self.read_disk_inode(|root| {
            if self.find_inode_id(new, root).is_some() {
                None
            } else {
                self.find_inode_id(old, root)
            }
        });
        let id = match id {
            Some(id) => id,
            None => return false,
        };
        self.modify_disk_inode(|root| {
            let count = root.size as usize / DIRENT_SZ;
            let mut slot = count;
            for i in 0..count {
                let mut entry = DirEntry::empty();
                root.read_at(i * DIRENT_SZ, entry.as_bytes_mut(), &self.block_device);
                if entry.name().is_empty() {
                    slot = i;
                    break;
                }
            }
            if slot == count {
                self.increase_size(((count + 1) * DIRENT_SZ) as u32, root, &mut fs);
            }
            root.write_at(
                slot * DIRENT_SZ,
                DirEntry::new(new, id).as_bytes(),
                &self.block_device,
            );
        });
        let (block, offset) = fs.get_disk_inode_pos(id);
        get_block_cache(block as usize, self.block_device.clone())
            .lock()
            .modify(offset, |inode: &mut DiskInode| inode.nlink += 1);
        block_cache_sync_all();
        true
    }

    /// Remove a directory entry and return its inode for deferred reclamation.
    pub fn unlink(&self, name: &str) -> Option<Arc<Inode>> {
        let fs = self.fs.lock();
        let found = self.read_disk_inode(|root| {
            let mut entry = DirEntry::empty();
            (0..root.size as usize / DIRENT_SZ).find_map(|i| {
                root.read_at(i * DIRENT_SZ, entry.as_bytes_mut(), &self.block_device);
                if !name.is_empty() && entry.name() == name {
                    Some((i, entry.inode_id()))
                } else {
                    None
                }
            })
        });
        let (slot, id) = found?;
        self.modify_disk_inode(|root| {
            root.write_at(
                slot * DIRENT_SZ,
                DirEntry::empty().as_bytes(),
                &self.block_device,
            );
        });
        let (block, offset) = fs.get_disk_inode_pos(id);
        get_block_cache(block as usize, self.block_device.clone())
            .lock()
            .modify(offset, |inode: &mut DiskInode| inode.nlink -= 1);
        block_cache_sync_all();
        drop(fs);
        Some(self.cached_inode(id))
    }

    /// Increase the size of a disk inode
    fn increase_size(
        &self,
        new_size: u32,
        disk_inode: &mut DiskInode,
        fs: &mut MutexGuard<EasyFileSystem>,
    ) {
        if new_size < disk_inode.size {
            return;
        }
        let blocks_needed = disk_inode.blocks_num_needed(new_size);
        let mut v: Vec<u32> = Vec::new();
        for _ in 0..blocks_needed {
            v.push(fs.alloc_data());
        }
        disk_inode.increase_size(new_size, v, &self.block_device);
    }
    /// Create inode under current inode by name
    pub fn create(&self, name: &str) -> Option<Arc<Inode>> {
        if name.is_empty() || name.len() > 27 {
            return None;
        }
        let mut fs = self.fs.lock();
        let op = |root_inode: &DiskInode| {
            // assert it is a directory
            assert!(root_inode.is_dir());
            // has the file been created?
            self.find_inode_id(name, root_inode)
        };
        if self.read_disk_inode(op).is_some() {
            return None;
        }
        // create a new file
        // alloc a inode with an indirect block
        let new_inode_id = fs.alloc_inode();
        // initialize inode
        let (new_inode_block_id, new_inode_block_offset) = fs.get_disk_inode_pos(new_inode_id);
        get_block_cache(new_inode_block_id as usize, Arc::clone(&self.block_device))
            .lock()
            .modify(new_inode_block_offset, |new_inode: &mut DiskInode| {
                new_inode.initialize(DiskInodeType::File);
            });
        self.modify_disk_inode(|root_inode| {
            // append file in the dirent
            let count = (root_inode.size as usize) / DIRENT_SZ;
            let mut file_count = count;
            for i in 0..count {
                let mut entry = DirEntry::empty();
                root_inode.read_at(i * DIRENT_SZ, entry.as_bytes_mut(), &self.block_device);
                if entry.name().is_empty() {
                    file_count = i;
                    break;
                }
            }
            let new_size = (count.max(file_count + 1)) * DIRENT_SZ;
            // increase size
            self.increase_size(new_size as u32, root_inode, &mut fs);
            // write dirent
            let dirent = DirEntry::new(name, new_inode_id);
            root_inode.write_at(
                file_count * DIRENT_SZ,
                dirent.as_bytes(),
                &self.block_device,
            );
        });

        block_cache_sync_all();
        drop(fs);
        Some(self.cached_inode(new_inode_id))
    }
    /// List inodes under current inode
    pub fn ls(&self) -> Vec<String> {
        let _fs = self.fs.lock();
        self.read_disk_inode(|disk_inode| {
            let file_count = (disk_inode.size as usize) / DIRENT_SZ;
            let mut v: Vec<String> = Vec::new();
            for i in 0..file_count {
                let mut dirent = DirEntry::empty();
                assert_eq!(
                    disk_inode.read_at(i * DIRENT_SZ, dirent.as_bytes_mut(), &self.block_device,),
                    DIRENT_SZ,
                );
                if !dirent.name().is_empty() {
                    v.push(String::from(dirent.name()));
                }
            }
            v
        })
    }
    /// Read data from current inode
    pub fn read_at(&self, offset: usize, buf: &mut [u8]) -> usize {
        let _fs = self.fs.lock();
        self.read_disk_inode(|disk_inode| disk_inode.read_at(offset, buf, &self.block_device))
    }
    /// Write data to current inode
    pub fn write_at(&self, offset: usize, buf: &[u8]) -> usize {
        let mut fs = self.fs.lock();
        let size = self.modify_disk_inode(|disk_inode| {
            self.increase_size((offset + buf.len()) as u32, disk_inode, &mut fs);
            disk_inode.write_at(offset, buf, &self.block_device)
        });
        block_cache_sync_all();
        size
    }
    /// Clear the data in current inode
    pub fn clear(&self) {
        let mut fs = self.fs.lock();
        self.modify_disk_inode(|disk_inode| {
            let size = disk_inode.size;
            let data_blocks_dealloc = disk_inode.clear_size(&self.block_device);
            assert!(data_blocks_dealloc.len() == DiskInode::total_blocks(size) as usize);
            for data_block in data_blocks_dealloc.into_iter() {
                fs.dealloc_data(data_block);
            }
        });
        block_cache_sync_all();
    }
}
