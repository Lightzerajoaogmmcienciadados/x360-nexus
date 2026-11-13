//! Educational Xenon/PPC64 interpreter. This is NOT a complete Xbox 360 CPU.
use crate::memory::Memory;

#[derive(Debug,Clone)]
pub struct Cpu {
    pub gpr:[u64;32], pub pc:usize, pub lr:u64, pub ctr:u64, pub cr:u32,
    pub xer:u32, pub halted:bool, pub fpr:[f64;32],
}
fn mask32(mb:u32,me:u32)->u32{
    let mut mask=0;
    for b in 0..32 {if (mb<=me && b>=mb && b<=me)||(mb>me && (b>=mb || b<=me)) {mask|=1u32 << (31-b);}}
    mask
}
impl Cpu {
    pub fn new()->Self {Self{gpr:[0;32],pc:0,lr:0,ctr:0,cr:0,xer:0,halted:false,fpr:[0.0;32]}}
    fn cr_field(&mut self, field:u32, bits:u32){let shift=28-4*field;self.cr=(self.cr&!(15u32<<shift))|((bits&15)<<shift);}
    fn cr_bit(&self,bit:u32)->bool{((self.cr>>(31-bit))&1)!=0}
    fn set_cr_bit(&mut self,bit:u32,value:bool){
        let mask=1u32<<(31-bit);
        if value{self.cr|=mask;}else{self.cr&=!mask;}
    }
    fn cr0(&mut self, val:u64) {
        let s=val as i64;
        let so=(self.xer>>31)&1;
        self.cr_field(0,(if s<0 {8} else if s>0 {4} else {2})|so);
    }
    fn cmp(&mut self,instr:u32,unsigned:bool){
        let f=(instr>>23)&7;let is64=(instr&(1<<21))!=0;let ra=((instr>>16)&31) as usize;
        let bits=if unsigned {
            let a=if is64 {self.gpr[ra]}else{self.gpr[ra] as u32 as u64};
            let b=if is64 {instr as u16 as u64}else{instr as u16 as u64};
            if a<b{8}else if a>b{4}else{2}
        } else {
            let a=if is64 {self.gpr[ra] as i64}else{self.gpr[ra] as i32 as i64};
            let b=instr as u16 as i16 as i64;
            if a<b{8}else if a>b{4}else{2}
        };self.cr_field(f,bits|((self.xer>>31)&1));
    }
    fn ea(&self,ra:usize,disp:i16)->Result<usize,String>{
        let base=if ra==0{0}else{self.gpr[ra]};
        let effective=base.wrapping_add(disp as i64 as u64);
        usize::try_from(effective).map_err(|_|"effective address overflow".into())
    }
    fn branch_condition(&mut self,bo:u32,bi:u32)->bool{
        let cr_ok=bo&16!=0 || self.cr_bit(bi)==(bo&8!=0);
        let ctr_ok=if bo&4!=0 {true}else{
            self.ctr=self.ctr.wrapping_sub(1);
            ((self.ctr!=0) != (bo&2!=0))
        };
        cr_ok && ctr_ok
    }
    pub fn step(&mut self,m:&Memory)->Result<(),String>{
        // Compatibility wrapper; stores require mutable memory, use step_mut.
        let mut local=Memory::with_base(m.read(m.base() as usize,m.len()).map_err(str::to_string)?,m.base());
        self.step_mut(&mut local)
    }
    pub fn step_mut(&mut self,mem:&mut Memory)->Result<(),String>{
        if self.halted{return Err("CPU halted".into());}
        if self.pc%4!=0{return Err(format!("misaligned PC 0x{:X}",self.pc));}
        let ins=mem.fetch_u32_be(self.pc).map_err(|e|format!("fetch @0x{:08X}: {e}",self.pc))?;
        let op=ins>>26;
        let rt=((ins>>21)&31) as usize;let ra=((ins>>16)&31) as usize;let rb=((ins>>11)&31) as usize;
        let imm=ins as u16 as i16;
        let mut next=self.pc.checked_add(4).ok_or("PC overflow")?;
        match op {
            7=>{self.gpr[rt]=(self.gpr[ra] as i64).wrapping_mul(imm as i64) as u64;},
            14=>{let base=if ra==0{0}else{self.gpr[ra]};self.gpr[rt]=base.wrapping_add(imm as i64 as u64);},
            15=>{let base=if ra==0{0}else{self.gpr[ra]};self.gpr[rt]=base.wrapping_add(((imm as i64)<<16) as u64);},
            24=>{self.gpr[ra]=self.gpr[rt] | (ins as u16 as u64);},
            26=>{self.gpr[ra]=self.gpr[rt] ^ (ins as u16 as u64);},
            27=>{self.gpr[ra]=self.gpr[rt] ^ ((ins as u16 as u64)<<16);},
            29=>{self.gpr[ra]=self.gpr[rt] & ((ins as u16 as u64)<<16);self.cr0(self.gpr[ra]);},
            28=>{self.gpr[ra]=self.gpr[rt] & (ins as u16 as u64);self.cr0(self.gpr[ra]);}, // andi.
            25=>{self.gpr[ra]=self.gpr[rt] | ((ins as u16 as u64)<<16);},
            20|21|23=>{ // rlwimi / rlwinm / rlwnm
                let sh=if op==23 {(self.gpr[rb] as u32)&31}else{(ins>>11)&31};
                let mb=(ins>>6)&31;let me=(ins>>1)&31;
                let mask=mask32(mb,me);
                let v=(self.gpr[rt] as u32).rotate_left(sh)&mask;
                self.gpr[ra]=if op==20{((self.gpr[ra] as u32 & !mask)|v) as u64}else{v as u64};
                if ins&1!=0{self.cr0(self.gpr[ra]);}
            },
            10|11=>self.cmp(ins,op==10),
            30=>{ // rldicl, rldicr: MD-form bit 1 encodes SH[5].
                let form=(ins>>2)&7;
                let sh=((ins>>11)&31)|((ins&2)<<4);
                let mask_end=((ins>>6)&31)|(ins&0x20);
                let value=self.gpr[rt].rotate_left(sh);
                self.gpr[ra]=match form{
                    0=>value & (u64::MAX>>mask_end), // rldicl: clear left
                    1=>value & (u64::MAX<<(63-mask_end)), // rldicr: clear right
                    _=>return Err(format!("opcode30 MD-form {form} unsupported")),
                };
                if ins&1!=0{self.cr0(self.gpr[ra]);}
            },
            8=>{ // subfic RT,RA,SIMM
                self.gpr[rt]=(imm as i64 as u64).wrapping_sub(self.gpr[ra]);
            },
            12|13=>{ // addic / addic.
                let a=self.gpr[ra];
                let b=imm as i64 as u64;
                self.gpr[rt]=a.wrapping_add(b);
                if op==13{self.cr0(self.gpr[rt]);}
            },
            32|33|34|36|37|38|40|44=>{
                let addr=self.ea(ra,imm)?;
                let update=op==33 || op==37;
                if update && ra==0{return Err("update form cannot use r0 as base".into());}
                if op==33 && ra==rt{return Err("lwzu RA cannot equal RT".into());}
                match op {
                    32|33=>{if addr%4!=0{return Err("unaligned lwz".into())};self.gpr[rt]=mem.read_u32_be(addr).map_err(str::to_string)? as u64;},
                    34=>{self.gpr[rt]=mem.read_u8(addr).map_err(str::to_string)? as u64;},
                    36|37=>{if addr%4!=0{return Err("unaligned stw".into())};mem.write_u32_be(addr,self.gpr[rt] as u32).map_err(str::to_string)?;},
                    38=>mem.write_u8(addr,self.gpr[rt] as u8).map_err(str::to_string)?,
                    40=>{if addr%2!=0{return Err("unaligned lhz".into())};self.gpr[rt]=mem.read_u16_be(addr).map_err(str::to_string)? as u64;},
                    44=>{if addr%2!=0{return Err("unaligned sth".into())};mem.write_u16_be(addr,self.gpr[rt] as u16).map_err(str::to_string)?;},
                    _=>unreachable!(),
                }
                if update{self.gpr[ra]=addr as u64;}
            },
            58|62=>{ // ld/std DS-form, update variants not yet supported
                let ds=(ins & 0xFFFC) as i16 as i64;
                let base=if ra==0{0}else{self.gpr[ra]};
                let addr=base.wrapping_add(ds as u64) as usize;
                if addr%8!=0{return Err("unaligned ld/std".into());}
                if ins&3!=0{return Err("ld/std update/other DS form not supported".into());}
                if op==58{self.gpr[rt]=mem.read_u64_be(addr).map_err(str::to_string)?;}
                else{mem.write_u64_be(addr,self.gpr[rt]).map_err(str::to_string)?;}
            },
            48|50|52|54=>{ // lfs, lfd, stfs, stfd
                let addr=self.ea(ra,imm)?;
                match op {
                    48=>{if addr%4!=0{return Err("unaligned lfs".into())};
                        self.fpr[rt]=f32::from_bits(mem.read_u32_be(addr).map_err(str::to_string)?) as f64;},
                    50=>{if addr%8!=0{return Err("unaligned lfd".into())};
                        self.fpr[rt]=f64::from_bits(mem.read_u64_be(addr).map_err(str::to_string)?);},
                    52=>{if addr%4!=0{return Err("unaligned stfs".into())};
                        mem.write_u32_be(addr,(self.fpr[rt] as f32).to_bits()).map_err(str::to_string)?;},
                    54=>{if addr%8!=0{return Err("unaligned stfd".into())};
                        mem.write_u64_be(addr,self.fpr[rt].to_bits()).map_err(str::to_string)?;},
                    _=>unreachable!(),
                }
            },
            18=>{
                let displacement=(((ins&0x03ff_fffc) as i32)<<6)>>6;
                let dst=if ins&2!=0{displacement as i64}else{(self.pc as i64).wrapping_add(displacement as i64)};
                if dst<0{return Err("negative branch target".into());}
                if ins&1!=0{self.lr=next as u64;}
                next=dst as usize;
            },
            16=>{
                let bo=(ins>>21)&31;let bi=(ins>>16)&31;
                let displacement=(((ins&0x0000_fffc) as i16) as i64);
                if self.branch_condition(bo,bi){
                    let dst=if ins&2!=0{displacement}else{(self.pc as i64).wrapping_add(displacement)};
                    if dst<0{return Err("negative bc target".into());}
                    next=dst as usize;
                }
                if ins&1!=0{self.lr=(self.pc+4) as u64;}
            },
            19=>{
                let xo=(ins>>1)&1023;let bo=(ins>>21)&31;let bi=(ins>>16)&31;
                match xo {
                    0=>{ // mcrf: field-to-field copy, no implicit CR0 update.
                        let bf=(ins>>23)&7;let bfa=(ins>>18)&7;
                        let bits=(self.cr>>(28-4*bfa))&15;
                        self.cr_field(bf,bits);
                    },
                    16|528=>{ // bclr / bcctr, LK, branch target from pre-link LR/CTR.
                        if xo==528 && bo&4==0{return Err("bcctr: BO requests invalid CTR decrement".into());}
                        let target=if xo==16{self.lr}else{self.ctr};
                        let taken=self.branch_condition(bo,bi);
                        if ins&1!=0{self.lr=next as u64;}
                        if taken {next=(target&!3) as usize;}
                    },
                    257|129|289|225|33|449|417|193=>{ // CR bitwise logic
                        let dst=(ins>>21)&31;let a=self.cr_bit((ins>>16)&31);
                        let b=self.cr_bit((ins>>11)&31);
                        let bit=match xo {
                            257=>a&b,129=>a&!b,289=>!(a^b),225=>!(a&b),
                            33=>!(a|b),449=>a|b,417=>a|!b,193=>a^b,_=>unreachable!()
                        };
                        self.set_cr_bit(dst,bit);
                    },
                    _=>return Err(format!("opcode 19 XO={xo} unsupported")),
                }
            },
            31=>{
                let xo=(ins>>1)&1023;
                match xo {
                    0|32=>{ // cmp / cmpl register
                        let field=(ins>>23)&7;let is64=ins&(1<<21)!=0;
                        let a=self.gpr[ra];let b=self.gpr[rb];
                        let bits=if xo==0 {
                            let (a,b)=if is64 {(a as i64,b as i64)} else {(a as i32 as i64,b as i32 as i64)};
                            if a<b{8}else if a>b{4}else{2}
                        }else{
                            let (a,b)=if is64 {(a,b)}else{(a as u32 as u64,b as u32 as u64)};
                            if a<b{8}else if a>b{4}else{2}
                        }; self.cr_field(field,bits);
                    },
                    19=>{self.gpr[rt]=self.cr as u64;}, // mfcr
                    144=>{ // mtcrf: FXM 8-bit mask chooses affected CR fields
                        let fxm=(ins>>12)&255;
                        for field in 0..8u32 {
                            if fxm&(0x80>>field)!=0 {
                                let shift=28-4*field;
                                self.cr_field(field,((self.gpr[rt] as u32)>>shift)&15);
                            }
                        }
                    },
                    266=>{self.gpr[rt]=self.gpr[ra].wrapping_add(self.gpr[rb]);if ins&1!=0{self.cr0(self.gpr[rt]);}},
                    40=>{self.gpr[rt]=self.gpr[rb].wrapping_sub(self.gpr[ra]);if ins&1!=0{self.cr0(self.gpr[rt]);}}, // subf
                    316=>{self.gpr[ra]=self.gpr[rt]^self.gpr[rb];if ins&1!=0{self.cr0(self.gpr[ra]);}},
                    28=>{self.gpr[ra]=self.gpr[rt]&self.gpr[rb];if ins&1!=0{self.cr0(self.gpr[ra]);}},
                    24=>{self.gpr[ra]=(self.gpr[rt] as u32).wrapping_shl(self.gpr[rb] as u32&31) as u64;},
                    23|151|87|215|279|407=>{
                        let base=if ra==0 {0} else {self.gpr[ra]};
                        let addr=base.wrapping_add(self.gpr[rb]);
                        let address=usize::try_from(addr).map_err(|_|"indexed address overflow")?;
                        match xo {
                            23=>{if addr%4!=0{return Err("unaligned lwzx".into())};self.gpr[rt]=mem.read_u32_be(address).map_err(str::to_string)? as u64},
                            151=>{if addr%4!=0{return Err("unaligned stwx".into())};mem.write_u32_be(address,self.gpr[rt] as u32).map_err(str::to_string)?},
                            87=>self.gpr[rt]=mem.read_u8(address).map_err(str::to_string)? as u64,
                            215=>mem.write_u8(address,self.gpr[rt] as u8).map_err(str::to_string)?,
                            279=>{if addr%2!=0{return Err("unaligned lhzx".into())};self.gpr[rt]=mem.read_u16_be(address).map_err(str::to_string)? as u64},
                            407=>{if addr%2!=0{return Err("unaligned sthx".into())};mem.write_u16_be(address,self.gpr[rt] as u16).map_err(str::to_string)?},
                            _=>unreachable!(),
                        }
                    },
                    444|412|124|476|284|60=>{ // or, orc, nor, nand, eqv, andc
                        let a=self.gpr[rt];let b=self.gpr[rb];
                        self.gpr[ra]=match xo{444=>a|b,412=>a|!b,124=>!(a|b),
                            476=>!(a&b),284=>!(a^b),60=>a&!b,_=>unreachable!()};
                        if ins&1!=0{self.cr0(self.gpr[ra]);}
                    },
                    26=>{self.gpr[ra]=(self.gpr[rt] as u32).leading_zeros() as u64;},
                    954=>{self.gpr[ra]=(self.gpr[rt] as i8 as i64) as u64;},
                    922=>{self.gpr[ra]=(self.gpr[rt] as i16 as i64) as u64;},
                    986=>{self.gpr[ra]=(self.gpr[rt] as i32 as i64) as u64;},
                    104=>{self.gpr[rt]=(self.gpr[ra] as i64).wrapping_neg() as u64;
                          if ins&1!=0{self.cr0(self.gpr[rt]);}},
                    536=>{let n=(self.gpr[rb] as u32)&63;
                          self.gpr[ra]=if n>31{0}else{((self.gpr[rt] as u32)>>n) as u64};},
                    27=>{let n=(self.gpr[rb]&127) as u32;
                         self.gpr[ra]=self.gpr[rt].checked_shl(n).unwrap_or(0);},
                    539=>{let n=(self.gpr[rb]&127) as u32;
                          self.gpr[ra]=self.gpr[rt].checked_shr(n).unwrap_or(0);},
                    235=>{self.gpr[rt]=(self.gpr[ra] as i32).wrapping_mul(self.gpr[rb] as i32) as i64 as u64;
                          if ins&1!=0{self.cr0(self.gpr[rt]);}},
                    233=>{self.gpr[rt]=self.gpr[ra].wrapping_mul(self.gpr[rb]);
                          if ins&1!=0{self.cr0(self.gpr[rt]);}},
                    339|467=>{
                        let spr=((ins>>16)&31)|(((ins>>11)&31)<<5);
                        if xo==339 {self.gpr[rt]=match spr{1=>self.xer as u64,8=>self.lr,9=>self.ctr,_=>return Err(format!("mfspr unsupported spr={spr}"))};}
                        else {match spr{1=>self.xer=self.gpr[rt] as u32,8=>self.lr=self.gpr[rt],9=>self.ctr=self.gpr[rt],_=>return Err(format!("mtspr unsupported spr={spr}"))};}
                    },
                    _=>return Err(format!("opcode31 XO={xo} unsupported")),
                }
            },
            63=>{ // Basic scalar floating point operations. No FPSCR exception model yet.
                let xo=(ins>>1)&1023;
                let form=xo&31;
                let fc=((ins>>6)&31) as usize;
                let a=self.fpr[ra];let b=self.fpr[rb];
                self.fpr[rt]=match (form,xo) {
                    (21,_)=>a+b, // fadd
                    (20,_)=>a-b, // fsub
                    (18,_)=>a/b, // fdiv
                    (25,_)=>a*self.fpr[fc], // fmul: FRC operand
                    (_,72)=>b, // fmr
                    _=>return Err(format!("FPU opcode63 XO={xo} unsupported")),
                };
            },
            17=>return Err(format!("sc: guest syscall, r0=0x{:X}",self.gpr[0])),
            _=>return Err(format!("unsupported opcode {op} at PC=0x{:08X} ins={ins:08X}",self.pc)),
        }
        self.pc=next;Ok(())
    }
    pub fn run_steps(&mut self,mem:&Memory,limit:usize)->Result<usize,String>{
        for i in 0..limit {self.step(mem)?;if self.halted{return Ok(i+1);}}
        Ok(limit)
    }
}
impl Default for Cpu{fn default()->Self{Self::new()}}
#[cfg(test)]mod tests {
use super::*;
#[test]fn arithmetic_and_memory(){
    let mut m=Memory::new(256);let mut c=Cpu::new();
    // addi r3,r0,5; stw r3,128(0); lwz r4,128(0); add r5,r3,r4
    m.write(0,&[0x38,0x60,0,5,0x90,0x60,0,128,0x80,0x80,0,128,0x7c,0xa3,0x22,0x14]).unwrap();
    for _ in 0..4{c.step_mut(&mut m).unwrap()};assert_eq!(c.gpr[5],10);assert_eq!(m.read_u32_be(128).unwrap(),5);
}
#[test]fn compare_and_bc(){
    let mut m=Memory::new(64);let mut c=Cpu::new();
    // addi r3,0,5; cmpwi cr0,r3,5; beq +8; addi r4,0,1; addi r4,0,9
    let code=[0x38,0x60,0,5,0x2c,0x03,0,5,0x41,0x82,0,8,0x38,0x80,0,1,0x38,0x80,0,9];
    m.write(0,&code).unwrap();for _ in 0..4 {c.step_mut(&mut m).unwrap()};assert_eq!(c.gpr[4],9);
}
#[test]fn mflr_mtlr_blr(){
    let mut m=Memory::new(64);let mut c=Cpu::new();
    m.write(0,&[0x7c,0x08,0x02,0xa6,0x7c,0x08,0x03,0xa6,0x4e,0x80,0,0x20]).unwrap();
    c.lr=8;c.step_mut(&mut m).unwrap();assert_eq!(c.gpr[0],8);c.lr=0;c.step_mut(&mut m).unwrap();assert_eq!(c.lr,8);c.step_mut(&mut m).unwrap();assert_eq!(c.pc,8);
}
#[test]fn unknown_safe(){let m=Memory::new(4);let mut c=Cpu::new();let err=c.step(&m).unwrap_err();assert!(err.contains("unsupported"));}
    #[test]fn indexed_load_store_and_fpu(){
        let mut m=Memory::new(128);let mut c=Cpu::new();
        c.gpr[3]=12;c.gpr[4]=16;c.gpr[5]=0x12345678;
        // stwx r5,r3,r4 ; lwzx r6,r3,r4
        let stwx=(31u32<<26)|(5<<21)|(3<<16)|(4<<11)|(151<<1);
        let lwzx=(31u32<<26)|(6<<21)|(3<<16)|(4<<11)|(23<<1);
        m.write(0,&stwx.to_be_bytes()).unwrap();m.write(4,&lwzx.to_be_bytes()).unwrap();
        c.step_mut(&mut m).unwrap();c.step_mut(&mut m).unwrap();
        assert_eq!(c.gpr[6],0x12345678);
        c.fpr[2]=1.5;c.fpr[3]=2.25;
        let fadd=(63u32<<26)|(4<<21)|(2<<16)|(3<<11)|(21<<1);
        m.write(8,&fadd.to_be_bytes()).unwrap();c.step_mut(&mut m).unwrap();
        assert_eq!(c.fpr[4],3.75);
    }
}
#[cfg(test)] mod v57_condition_register_regression {
    use super::*;
    fn xform(op:u32,rd:u32,ra:u32,rb:u32,xo:u32)->u32 {
        (op<<26)|(rd<<21)|(ra<<16)|(rb<<11)|(xo<<1)
    }
    #[test] fn minecraft_first_opcode_is_a_real_conditional_branch(){
        let pc=0x8214_AB10usize;let op=0x419A_0088u32; // bc BO=12 BI=26 +0x88
        let mut m=Memory::with_base(vec![0;0x100],pc as u64);
        m.write(pc,&op.to_be_bytes()).unwrap();
        let mut cpu=Cpu::new();cpu.pc=pc;
        cpu.step_mut(&mut m).unwrap();assert_eq!(cpu.pc,pc+4); // CR6 bit 2 false
        cpu.pc=pc;cpu.cr|=1<<(31-26);
        cpu.step_mut(&mut m).unwrap();assert_eq!(cpu.pc,pc+0x88); // CR6 bit 2 true
    }
    #[test] fn compare_so_and_field_logic_are_bit_accurate(){
        let mut mem=Memory::new(128);let mut cpu=Cpu::new();
        cpu.xer=0x8000_0000;cpu.gpr[3]=5;
        let cmpi=(11u32<<26)|(6<<23)|(3<<16)|5; // cmpwi cr6,r3,5
        mem.write(0,&cmpi.to_be_bytes()).unwrap();
        cpu.step_mut(&mut mem).unwrap();assert_eq!((cpu.cr>>4)&15,3); // EQ + SO
        let crxor=(19u32<<26)|(26<<21)|(26<<16)|(26<<11)|(193<<1);
        mem.write(4,&crxor.to_be_bytes()).unwrap();
        cpu.step_mut(&mut mem).unwrap();assert!(!cpu.cr_bit(26));
        let cror=(19u32<<26)|(26<<21)|(25<<16)|(25<<11)|(449<<1);
        mem.write(8,&cror.to_be_bytes()).unwrap();
        cpu.step_mut(&mut mem).unwrap();assert!(cpu.cr_bit(26)); // EQ bit from cr6
        let mcrf=(19u32<<26)|(3<<23)|(6<<18);
        mem.write(12,&mcrf.to_be_bytes()).unwrap();
        cpu.step_mut(&mut mem).unwrap();assert_eq!((cpu.cr>>16)&15,(cpu.cr>>4)&15);
    }
    #[test] fn rldicr_and_logical_xor_execute(){
        let mut mem=Memory::new(32);let mut cpu=Cpu::new();
        cpu.gpr[3]=0x1234_5678_9ABC_DEF0;
        // rldicr r4,r3,8,31: MD-form XO=1, ME=31
        let ins=(30u32<<26)|(3<<21)|(4<<16)|(8<<11)|(31<<6)|(1<<2);
        mem.write(0,&ins.to_be_bytes()).unwrap();
        cpu.step_mut(&mut mem).unwrap();
        assert_eq!(cpu.gpr[4],cpu.gpr[3].rotate_left(8)&0xFFFF_FFFF_0000_0000);
        let xori=(26u32<<26)|(3<<21)|(5<<16)|0xFF;
        mem.write(4,&xori.to_be_bytes()).unwrap();
        cpu.step_mut(&mut mem).unwrap();assert_eq!(cpu.gpr[5],0x1234_5678_9ABC_DE0F);
    }
}
