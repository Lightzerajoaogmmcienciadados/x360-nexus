//! Bounded experimental guest virtual address mapping for original PPC diagnostics.
//! NOT an Xbox 360 MMU, TLB, real Xenon page table or game image mapping.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Permissions(u8);
impl Permissions {
    pub const READ: Self = Self(1);
    pub const WRITE: Self = Self(2);
    pub const EXECUTE: Self = Self(4);
    pub const RW: Self = Self(3);
    pub const RX: Self = Self(5);
    pub fn can_read(self)->bool{self.0&1!=0}
    pub fn can_write(self)->bool{self.0&2!=0}
    pub fn can_execute(self)->bool{self.0&4!=0}
}
#[derive(Debug, Clone)]
pub struct GuestMapping {
    pub guest_base: usize,
    pub physical_base: usize,
    pub size: usize,
    pub perms: Permissions,
}
#[derive(Debug)]
pub struct Memory {
    data: Vec<u8>,
    mappings: Vec<GuestMapping>,
}
impl Memory {
    pub fn new(bytes:usize)->Self { Self{data:vec![0;bytes],mappings:Vec::new()} }
    pub fn len(&self)->usize{self.data.len()}
    pub fn mappings(&self)->&[GuestMapping]{&self.mappings}
    /// Maps a page-aligned guest virtual interval onto a bounded host backing region.
    /// The backing array is owned by the emulator; no host pointer is exposed.
    pub fn map(&mut self, guest_base:usize, physical_base:usize, size:usize, perms:Permissions)
      ->Result<(), &'static str> {
        const PAGE:usize=4096;
        if size==0||size%PAGE!=0||guest_base%PAGE!=0||physical_base%PAGE!=0 {
            return Err("mapping must align to 4096 bytes");
        }
        if !perms.can_read(){return Err("mapping must permit reads")}
        let guest_end=guest_base.checked_add(size).ok_or("guest address overflow")?;
        let physical_end=physical_base.checked_add(size).ok_or("physical address overflow")?;
        if physical_end>self.data.len(){return Err("physical mapping out of bounds")}
        if self.mappings.len()>=128{return Err("too many mapped regions")}
        for old in &self.mappings{
            let end=old.guest_base+old.size;
            if guest_base<end&&old.guest_base<guest_end{return Err("overlapping guest mapping")}
        }
        self.mappings.push(GuestMapping{guest_base,physical_base,size,perms});
        Ok(())
    }
    fn translate(&self, addr:usize,len:usize,mode:u8)->Result<usize,&'static str> {
        let end=addr.checked_add(len).ok_or("address overflow")?;
        for map in &self.mappings{
            let limit=map.guest_base+map.size;
            if addr>=map.guest_base && addr<limit {
                if end>limit{return Err("crosses virtual region boundary")}
                if map.perms.0 & mode != mode {return Err("guest memory permission denied")}
                let offset=addr-map.guest_base;
                return map.physical_base.checked_add(offset).ok_or("physical overflow");
            }
        }
        // Legacy low physical addresses remain valid for diagnostic programs.
        // High virtual addresses must be explicitly mapped.
        if end<=self.data.len() { Ok(addr) } else { Err("unmapped or out-of-range address") }
    }
    pub fn read_u32_be(&self,addr:usize)->Result<u32,&'static str> {
        let b=self.read(addr,4)?;
        Ok(u32::from_be_bytes(b.try_into().unwrap()))
    }
    pub fn read_exec_u32_be(&self,addr:usize)->Result<u32,&'static str> {
        let offset=self.translate(addr,4,Permissions::EXECUTE.0)?;
        let b=self.data.get(offset..offset.checked_add(4).ok_or("address overflow")?)
          .ok_or("memory out of range")?;
        Ok(u32::from_be_bytes(b.try_into().unwrap()))
    }
    pub fn write(&mut self,addr:usize,src:&[u8])->Result<(),&'static str> {
        let offset=self.translate(addr,src.len(),Permissions::WRITE.0)?;
        let dst=self.data.get_mut(offset..offset.checked_add(src.len()).ok_or("address overflow")?)
          .ok_or("memory out of range")?;
        dst.copy_from_slice(src);Ok(())
    }
    pub fn read(&self,addr:usize,len:usize)->Result<&[u8],&'static str> {
        let offset=self.translate(addr,len,Permissions::READ.0)?;
        self.data.get(offset..offset.checked_add(len).ok_or("address overflow")?)
          .ok_or("memory out of range")
    }
}
#[cfg(test)] mod tests {
    use super::*;
    #[test]fn guest_virtual_mapping_read_execute(){
        let mut mem=Memory::new(2*4096);
        mem.write(0,&[0x38,0x60,0x00,0x2A]).unwrap();
        mem.map(0x8200_0000,0,4096,Permissions::RX).unwrap();
        assert_eq!(mem.read_exec_u32_be(0x8200_0000).unwrap(),0x3860002A);
        assert_eq!(mem.read(0x8200_0000,4).unwrap(),&[0x38,0x60,0,42]);
        assert!(mem.write(0x8200_0000,&[0]).is_err());
    }
    #[test]fn rejects_overlap_and_out_of_range(){
        let mut mem=Memory::new(8192);
        mem.map(0x8200_0000,0,4096,Permissions::RW).unwrap();
        assert!(mem.map(0x8200_0000,4096,4096,Permissions::RW).is_err());
        assert!(mem.map(0x8300_0000,8192,4096,Permissions::RW).is_err());
        assert!(mem.read_exec_u32_be(0x8200_0000).is_err());
        assert!(mem.read(0x8200_0fff,2).is_err());
    }
    #[test]fn cpu_executes_from_high_virtual_pc(){
        use crate::cpu::Cpu;
        let mut mem=Memory::new(8192);
        mem.write(0,&[0x38,0x60,0,42]).unwrap();
        mem.map(0x8200_0000,0,4096,Permissions::RX).unwrap();
        let mut cpu=Cpu::new();cpu.pc=0x8200_0000;cpu.step(&mut mem).unwrap();
        assert_eq!(cpu.pc,0x8200_0004);assert_eq!(cpu.gpr[3],42);
    }
}
