#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]
//! X360 GALAXY v5.8 Windows front-end.
//! This is a real, native x64 program linked with the Rust Xenon interpreter.
//! Commercial XEX2 decryption, Xenos shaders and Xbox kernel are not yet
//! integrated into this desktop executable; it does not claim game boot.
use eframe::egui::{self, Color32, RichText, ScrollArea, Vec2};
use std::{
    fs::{self, File},
    io::Read,
    path::{Path, PathBuf},
};
use x360nexus::{cpu::Cpu, loader, memory::Memory};

const GREEN: Color32 = Color32::from_rgb(22, 218, 90);
const MUTED: Color32 = Color32::from_rgb(150, 163, 158);
const MAX_TEST_BYTES: usize = 8 * 1024 * 1024;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Section { Library, Cpu, Diagnostics }
#[derive(Clone)]
struct Game { path: PathBuf, name: String, ext: String }
impl Game {
    fn new(path: PathBuf) -> Self {
        let name = path.file_name().map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "(sem nome)".to_string());
        let ext = path.extension().and_then(|x| x.to_str())
            .unwrap_or("").to_ascii_lowercase();
        Self {path,name,ext}
    }
    fn kind(&self) -> &'static str {
        match self.ext.as_str() {
            "iso"|"xiso" => "Xbox 360 ISO",
            "xex" => "Xbox 360 XEX2",
            "bin"|"ppc" => "PowerPC bruto (teste)",
            _ => "Arquivo desconhecido",
        }
    }
}
struct TestProgram {
    path: PathBuf,
    cpu: Cpu,
    mem: Memory,
    steps: u64,
    fault: Option<String>,
}
impl TestProgram {
    fn load(path: &Path) -> Result<Self, String> {
        let metadata = fs::metadata(path).map_err(|e|format!("Arquivo: {e}"))?;
        if metadata.len()<4 || metadata.len()>MAX_TEST_BYTES as u64 || metadata.len()%4!=0 {
            return Err(format!("Programa PPC: tamanho entre 4 bytes e {} MiB, alinhado a DWORD",
                MAX_TEST_BYTES/1024/1024));
        }
        let bytes = fs::read(path).map_err(|e|format!("Leitura PPC: {e}"))?;
        let mut mem = Memory::with_base(vec![0;bytes.len().max(4096)],0);
        mem.write(0,&bytes).map_err(str::to_string)?;
        let cpu=Cpu::new();
        Ok(Self{path:path.to_owned(),cpu,mem,steps:0,fault:None})
    }
    fn execute(&mut self,limit:u64) -> String {
        if let Some(reason)=&self.fault {
            return format!("Programa pausado: {reason}. Use REINICIAR.");
        }
        let mut executed=0u64;
        for _ in 0..limit.min(10_000) {
            let pc=self.cpu.pc;
            match self.cpu.step_mut(&mut self.mem) {
                Ok(())=>{self.steps+=1;executed+=1;},
                Err(e)=>{
                    self.fault=Some(format!("PC=0x{pc:08X}: {e}"));
                    break;
                }
            }
        }
        format!("PPC: +{executed} instrucoes; total={}; PC=0x{:08X}; {}",
            self.steps,self.cpu.pc,self.fault.as_deref().unwrap_or("executando"))
    }
}
struct GalaxyApp {
    page: Section,
    library: Vec<Game>,
    selected: Option<usize>,
    selected_info: String,
    cpu_test: Option<TestProgram>,
    logs: Vec<String>,
}
impl Default for GalaxyApp {
    fn default() -> Self {
        Self{
            page:Section::Library,library:Vec::new(),selected:None,
            selected_info:"Importe uma ISO/XEX para inspecionar ou um .ppc/.bin para testar a CPU.".into(),
            cpu_test:None,
            logs:vec![
                "X360 GALAXY v5.8 · Windows x64".into(),
                "Nucleo Rust PowerPC conectado; biblioteca XDVDFS/XEX2 em modo leitura.".into(),
                "IMPORTANTE: Minecraft/Forza retail ainda nao iniciam nesta versao Windows.".into(),
            ],
        }
    }
}
impl GalaxyApp {
    fn log(&mut self,msg:impl Into<String>) {
        self.logs.push(msg.into());
        if self.logs.len()>300 {
            let trim=self.logs.len()-300;
            self.logs.drain(..trim);
        }
    }
    fn import_files(&mut self){
        let files=rfd::FileDialog::new()
            .set_title("Importar arquivos locais Xbox 360 / PowerPC")
            .add_filter("Xbox 360 e PowerPC", &["iso","xiso","xex","ppc","bin"])
            .pick_files();
        if let Some(paths)=files {
            for path in paths {
                if let Some(index)=self.library.iter().position(|g|g.path==path) {
                    self.selected=Some(index);continue;
                }
                let name=path.display().to_string();
                self.library.push(Game::new(path));
                self.selected=Some(self.library.len()-1);
                self.log(format!("Adicionado: {name}"));
            }
        }
    }
    fn inspect_file(&mut self,path:&Path){
        let result=(||->Result<String,String>{
            let file=Game::new(path.to_owned());
            match file.ext.as_str(){
                "iso"|"xiso"=>{
                    let mut handle=File::open(path).map_err(|e|e.to_string())?;
                    let image=loader::inspect_iso(&mut handle)?;
                    let entries=loader::root_entries(&mut handle,&image)?;
                    let mut lines=vec![
                        format!("XDVDFS identificado: {}",image.layout),
                        format!("Imagem: {} bytes",image.image_size),
                        format!("Particao: 0x{:X}",image.partition),
                        format!("Raiz: setor {}, {} bytes",image.root_sector,image.root_size),
                        format!("{} entradas na raiz:",entries.len()),
                    ];
                    for entry in entries.iter().take(32){
                        lines.push(format!("{} {} · {} bytes",if entry.is_directory{"[DIR]"}else{"[FILE]"},entry.name,entry.size));
                    }
                    lines.push("Execucao de jogos Xbox 360 ainda indisponivel no Windows.".into());
                    Ok(lines.join("\n"))
                },
                "xex"=>{
                    let mut handle=File::open(path).map_err(|e|e.to_string())?;
                    let mut buf=vec![0u8;4096];
                    let read=handle.read(&mut buf).map_err(|e|e.to_string())?;
                    buf.truncate(read);
                    let info=loader::inspect_xex(&buf)?;
                    Ok(format!("XEX2 identificado\nFlags: 0x{:08X}\nPE offset: 0x{:08X}\nSecurity offset: 0x{:08X}\nHeaders opcionais: {}\n\nO loader AES/LZX e o kernel HLE da versao Android ainda nao foram conectados ao executavel Windows.",info.module_flags,info.header_size,info.security_offset,info.optional_headers))
                },
                "bin"|"ppc"=>Ok("Codigo PowerPC bruto. Use CARREGAR NA CPU para executar a partir de PC=0 no interpretador Rust v5.8. Arquivos XEX2 comerciais nao sao codigos brutos.".into()),
                _=>Err("Extensao nao reconhecida".into()),
            }
        })();
        match result{
            Ok(info)=>{self.selected_info=info;self.log("Inspecao concluida.");},
            Err(error)=>{self.selected_info=format!("Nao foi possivel analisar: {error}");self.log(format!("Inspecao: {error}"));}
        }
    }
    fn load_test(&mut self,path:&Path){
        match TestProgram::load(path){
            Ok(cpu)=>{self.cpu_test=Some(cpu);self.page=Section::Cpu;self.log("Binario PowerPC carregado no interpretador v5.8.");},
            Err(e)=>self.log(format!("Falha carregando PPC: {e}")),
        }
    }
    fn run_test(&mut self,count:u64){
        let result=match &mut self.cpu_test {
            Some(s)=>s.execute(count),
            None=>"Nenhum .ppc/.bin carregado. Importe um arquivo de codigo PowerPC.".into(),
        };
        self.log(result);
    }
    fn reset_test(&mut self){
        let path=self.cpu_test.as_ref().map(|x|x.path.clone());
        if let Some(path)=path{self.load_test(&path);}
    }
    fn library_page(&mut self,ui:&mut egui::Ui){
        ui.horizontal(|ui|{
            ui.heading(RichText::new("Biblioteca Xbox 360").color(GREEN).size(24.0));
            if ui.button("＋ IMPORTAR ISO / XEX / PPC").clicked(){self.import_files();}
        });
        ui.add_space(7.0);
        ui.colored_label(MUTED,"Inspecao local real de XDVDFS/XEX2; boot de jogos comerciais nao implementado nesta build Windows.");
        ui.separator();
        ui.columns(2,|columns|{
            columns[0].heading("Arquivos importados");
            ScrollArea::vertical().max_height(350.0).show(&mut columns[0],|ui|{
                if self.library.is_empty(){ui.label("Nenhum arquivo adicionado.");}
                for (index,game) in self.library.iter().enumerate(){
                    if ui.selectable_label(self.selected==Some(index),
                        format!("{}\n{}",game.name,game.kind())).clicked(){
                        self.selected=Some(index);
                    }
                }
            });
            columns[1].heading("Detalhes");
            let selected=self.selected.and_then(|i|self.library.get(i)).cloned();
            if let Some(game)=selected {
                columns[1].label(RichText::new(game.name).strong());
                columns[1].small(game.path.display().to_string());
                columns[1].horizontal(|ui|{
                    if ui.button("INSPECIONAR").clicked(){self.inspect_file(&game.path);}
                    if matches!(game.ext.as_str(),"ppc"|"bin"){
                        if ui.button("CARREGAR NA CPU").clicked(){self.load_test(&game.path);}
                    }else{
                        ui.add_enabled(false,egui::Button::new("INICIAR JOGO (NAO SUPORTADO)"));
                    }
                });
            }else{
                columns[1].label("Selecione um arquivo da biblioteca.");
            }
            ScrollArea::vertical().max_height(310.0).show(&mut columns[1],|ui|{
                ui.label(&self.selected_info);
            });
        });
    }
    fn cpu_page(&mut self,ui:&mut egui::Ui){
        ui.heading(RichText::new("CPU Xenon · Interpretador PowerPC").color(GREEN).size(24.0));
        ui.colored_label(MUTED,"Executa programas brutos de teste no mesmo nucleo Rust usado no Android. Nao executa XEX diretamente.");
        ui.separator();
        ui.horizontal(|ui|{
            if ui.button("PASSO").clicked(){self.run_test(1);}
            if ui.button("100 PASSOS").clicked(){self.run_test(100);}
            if ui.button("10.000 PASSOS").clicked(){self.run_test(10_000);}
            if ui.button("REINICIAR").clicked(){self.reset_test();}
            if ui.button("AUTOTESTE PPC").clicked(){
                match x360nexus::galaxy::run_demo(){
                    Ok(report)=>self.log(format!("PPC: {report}")),
                    Err(e)=>self.log(format!("Autoteste falhou: {e}")),
                }
            }
        });
        if let Some(test)=&self.cpu_test{
            ui.label(format!("Arquivo: {}",test.path.display()));
            ui.label(format!("PC=0x{:08X}  LR=0x{:016X}  CTR=0x{:016X}  CR=0x{:08X}  instrucoes={}",
                test.cpu.pc,test.cpu.lr,test.cpu.ctr,test.cpu.cr,test.steps));
            if let Some(err)=&test.fault{ui.colored_label(Color32::LIGHT_RED,err);}
            egui::Grid::new("ppc-registers").num_columns(4).striped(true).show(ui,|ui|{
                for i in 0..32{
                    ui.monospace(format!("r{i:02}=0x{:016X}",test.cpu.gpr[i]));
                    if i%4==3{ui.end_row();}
                }
            });
        }else{
            ui.label("Para executar: importe um arquivo .ppc/.bin e clique CARREGAR NA CPU.");
        }
    }
    fn diagnostics_page(&mut self,ui:&mut egui::Ui){
        ui.heading(RichText::new("Diagnosticos e estado do projeto").color(GREEN).size(24.0));
        ui.label("Ativos no Windows x64: GUI egui, biblioteca ISO XDVDFS, metadados XEX2, CPU PowerPC interpretada e memoria Big-Endian.");
        ui.label("Ainda nao portados: decrypt/decompress XEX2 comercial, vinculacao kernel HLE para jogo Windows, GPU Xenos/EDRAM/GLSL e audio de jogo.");
        ui.separator();
        ui.horizontal(|ui|{
            ui.heading("Registro");
            if ui.button("LIMPAR").clicked(){self.logs.clear();}
            if ui.button("COPIAR").clicked(){ui.ctx().copy_text(self.logs.join("\n"));}
        });
        ScrollArea::vertical().max_height(420.0).stick_to_bottom(true).show(ui,|ui|{
            for message in &self.logs{ui.monospace(message);}
        });
    }
}
impl eframe::App for GalaxyApp {
    fn update(&mut self,ctx:&egui::Context,_frame:&mut eframe::Frame){
        let mut visuals=egui::Visuals::dark();
        visuals.selection.bg_fill=Color32::from_rgb(17,94,47);
        visuals.widgets.active.bg_fill=Color32::from_rgb(15,101,43);
        visuals.widgets.hovered.bg_fill=Color32::from_rgb(24,77,45);
        ctx.set_visuals(visuals);
        egui::TopBottomPanel::top("top").show(ctx,|ui|{
            ui.horizontal(|ui|{
                ui.label(RichText::new("X360: GALAXY").color(GREEN).size(28.0).strong());
                ui.separator();
                ui.label(RichText::new("WINDOWS  ·  v5.8").color(MUTED));
            });
        });
        egui::SidePanel::left("sidebar").min_width(165.0).show(ctx,|ui|{
            ui.add_space(12.0);
            if ui.selectable_label(self.page==Section::Library,"▣  BIBLIOTECA").clicked(){self.page=Section::Library;}
            if ui.selectable_label(self.page==Section::Cpu,"⚙  CPU PPC").clicked(){self.page=Section::Cpu;}
            if ui.selectable_label(self.page==Section::Diagnostics,"≡  LOG / STATUS").clicked(){self.page=Section::Diagnostics;}
            ui.add_space(20.0);
            ui.colored_label(MUTED,"Xbox 360 em construcao");
            ui.small("Emulacao comercial incompleta.");
        });
        egui::CentralPanel::default().show(ctx,|ui|{
            ui.add_space(14.0);
            match self.page {
                Section::Library=>self.library_page(ui),
                Section::Cpu=>self.cpu_page(ui),
                Section::Diagnostics=>self.diagnostics_page(ui),
            }
        });
        egui::TopBottomPanel::bottom("foot").show(ctx,|ui|{
            ui.horizontal(|ui|{
                ui.colored_label(GREEN,"● NUCLEO RUST");
                ui.label("PPC interpretado | XEX2 metadata | XDVDFS");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center),|ui|{
                    ui.colored_label(MUTED,"Xenos e kernel retail pendentes");
                });
            });
        });
    }
}
fn main()->eframe::Result<()>{
    let options=eframe::NativeOptions{
        viewport:egui::ViewportBuilder::default()
            .with_title("X360 GALAXY v5.8 — Windows")
            .with_inner_size(Vec2::new(1120.0,720.0))
            .with_min_inner_size(Vec2::new(800.0,520.0)),
        ..Default::default()
    };
    eframe::run_native("X360 GALAXY",options,
        Box::new(|_cc|Ok(Box::new(GalaxyApp::default()))))
}
