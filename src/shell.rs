use crate::board::PinRef;
use crate::boards::{BoardImpl, board_from_str};
use crate::project::{Project, Wire};
use crate::render::{render_project, render_runtime_frame};
use crate::sim::RunningSim;
use anyhow::{Context, Result, anyhow, bail};
use rustyline::error::ReadlineError;
use std::io::{self, BufRead, IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn cmd_shell(start_dir: &Path, no_tui: bool) -> Result<()> {
    let root = Project::find_project_root(start_dir)?;
    let project = Project::load(&root.join("moxin.toml"))?;

    let comp_count = project.components.len();
    let wire_count = project.wires.len();
    let summary = if comp_count == 0 && wire_count == 0 {
        "empty project".to_string()
    } else {
        format!("{} components, {} wires", comp_count, wire_count)
    };
    println!(
        "welcome to moxin shell · board={} · {}",
        project.project.board, summary
    );

    let board = board_from_str(&project.project.board)?;
    let mut shell = Shell { root, project, running: None, board };

    let is_tty = io::stdin().is_terminal();
    if is_tty && !no_tui {
        crate::tui::run(&mut shell)?;
    } else if is_tty {
        repl_interactive(&mut shell)?;
    } else {
        repl_piped(&mut shell)?;
    }

    if let Some(sim) = shell.running.take() {
        sim.stop();
    }
    Ok(())
}

fn repl_interactive(shell: &mut Shell) -> Result<()> {
    let mut rl = rustyline::DefaultEditor::new().context("rustyline init")?;
    loop {
        let line = match rl.readline("moxin> ") {
            Ok(l) => l,
            Err(ReadlineError::Eof) | Err(ReadlineError::Interrupted) => { println!(); break; }
            Err(e) => { eprintln!("readline error: {}", e); break; }
        };
        let trimmed = line.trim();
        if trimmed.is_empty() { continue; }
        let _ = rl.add_history_entry(trimmed);
        if trimmed == "exit" || trimmed == "quit" { break; }
        match shell.dispatch(trimmed) {
            Ok(msg) if !msg.is_empty() => println!("{}", msg),
            Ok(_) => {}
            Err(e) => eprintln!("error: {}", e),
        }
    }
    Ok(())
}

fn repl_piped(shell: &mut Shell) -> Result<()> {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut out = stdout.lock();
    let mut handle = stdin.lock();
    let mut buf = String::new();
    loop {
        write!(out, "moxin> ")?;
        out.flush()?;
        buf.clear();
        match handle.read_line(&mut buf) {
            Ok(0) => break,
            Ok(_) => {}
            Err(_) => break,
        }
        let trimmed = buf.trim().to_string();
        if trimmed.is_empty() { continue; }
        writeln!(out, "{}", trimmed)?;
        out.flush()?;
        if trimmed == "exit" || trimmed == "quit" { break; }
        match shell.dispatch(&trimmed) {
            Ok(msg) if !msg.is_empty() => writeln!(out, "{}", msg)?,
            Ok(_) => {}
            Err(e) => writeln!(out, "error: {}", e)?,
        }
        out.flush()?;
    }
    Ok(())
}

pub struct Shell {
    root: PathBuf,
    pub project: Project,
    pub running: Option<RunningSim>,
    pub board: Box<dyn BoardImpl>,
}

impl Drop for Shell {
    fn drop(&mut self) {
        if let Some(sim) = self.running.take() {
            sim.stop();
        }
    }
}

impl Shell {
    pub fn dispatch(&mut self, line: &str) -> Result<String> {
        if let Some(rest) = line.strip_prefix("wire") {
            return self.cmd_wire(rest.trim());
        }
        let mut tokens = line.split_whitespace();
        let head = tokens.next().unwrap_or("");
        let rest: Vec<&str> = tokens.collect();
        match head {
            "board" => self.cmd_board(&rest),
            "add" => self.cmd_add(&rest),
            "show" => self.cmd_show(&rest),
            "edit" => self.cmd_edit(),
            "build" => self.cmd_build(),
            "run" => self.cmd_run_sim(),
            "adc" => self.cmd_adc(&rest),
            "dist" => self.cmd_dist(&rest),
            "env" => self.cmd_env(&rest),
            "ir" => self.cmd_ir(&rest),
            "send" => self.cmd_send(line),
            "stop" => self.cmd_stop(),
            "sleep" => self.cmd_sleep(&rest),
            "help" | "?" => Ok(help_text()),
            _ => bail!("unknown command: {} (try `help`)", head),
        }
    }

    fn cmd_board(&self, rest: &[&str]) -> Result<String> {
        match rest.first().copied() {
            Some("info") | None => Ok(self.board.spec().board_info_string()),
            Some(other) => bail!("unknown board subcommand: {}", other),
        }
    }

    fn cmd_add(&mut self, args: &[&str]) -> Result<String> {
        if args.is_empty() {
            bail!("usage: add <type> [color] --id <id>");
        }
        let kind = args[0].to_lowercase();
        let mut id: Option<String> = None;
        let mut positional: Vec<String> = Vec::new();
        let mut i = 1;
        while i < args.len() {
            let a = args[i];
            if a == "--id" {
                i += 1;
                if i >= args.len() { bail!("--id requires a value"); }
                id = Some(args[i].to_string());
            } else if let Some(v) = a.strip_prefix("--id=") {
                id = Some(v.to_string());
            } else {
                positional.push(a.to_string());
            }
            i += 1;
        }
        let id = id.ok_or_else(|| anyhow!("--id required"))?;

        let reg = crate::components::registry();
        let def = reg
            .resolve(&kind)
            .ok_or_else(|| anyhow!("unknown component type: {}", kind))?;
        let comp = def.build(id, &positional)?;
        let display = def.display_after_add(&comp);

        self.project.add_component(comp)?;
        self.save_project()?;
        Ok(format!("✓ added {}", display))
    }

    fn cmd_wire(&mut self, rest: &str) -> Result<String> {
        let parts: Vec<&str> = rest.split("->").map(|s| s.trim()).collect();
        if parts.len() != 2 || parts[0].is_empty() || parts[1].is_empty() {
            bail!("usage: wire <from> -> <to>");
        }
        let from_ref = PinRef::parse(parts[0])?;
        let to_ref = PinRef::parse(parts[1])?;
        if let PinRef::Component { id, .. } = &from_ref {
            if !self.project.components.iter().any(|c| &c.id == id) {
                bail!("unknown component id: {}", id);
            }
        }
        if let PinRef::Component { id, .. } = &to_ref {
            if !self.project.components.iter().any(|c| &c.id == id) {
                bail!("unknown component id: {}", id);
            }
        }
        let spec = self.board.spec();
        if !spec.pin_ref_valid(&from_ref) {
            bail!("unknown pin {} on {} — run `board info` for valid pins", from_ref.render(), spec.board_id);
        }
        if !spec.pin_ref_valid(&to_ref) {
            bail!("unknown pin {} on {} — run `board info` for valid pins", to_ref.render(), spec.board_id);
        }
        let from_canon = from_ref.render_canonical();
        let to_canon = to_ref.render_canonical();
        self.project.add_wire(Wire { from: from_canon.clone(), to: to_canon.clone() });
        self.save_project()?;
        Ok(format!("✓ wired {} -> {}", from_canon, to_canon))
    }

    fn cmd_show(&self, args: &[&str]) -> Result<String> {
        match args.first().copied() {
            Some("project") => Ok(render_project(&self.project)),
            None => {
                if let Some(sim) = self.running.as_ref() {
                    let Ok(st) = sim.state.lock() else {
                        return Ok("(state unavailable)".to_string());
                    };
                    Ok(render_runtime_frame(&self.project, &st, self.board.spec()))
                } else {
                    Ok(format!("{}\n(simulator not running — try `run`)", render_project(&self.project)))
                }
            }
            Some(other) => bail!("unknown show subcommand: {}", other),
        }
    }

    fn cmd_edit(&self) -> Result<String> {
        let editor = std::env::var("EDITOR").unwrap_or_else(|_| {
            if cfg!(windows) { "notepad".to_string() } else { "vi".to_string() }
        });
        let target = self.root.join(
            self.project.code.as_ref().map(|c| c.src.clone())
                .unwrap_or_else(|| "src/main.ino".into()),
        );
        let status = Command::new(&editor).arg(&target).status()
            .with_context(|| format!("spawn $EDITOR ({})", editor))?;
        if !status.success() { bail!("editor exited non-zero"); }
        Ok(String::new())
    }

    fn cmd_build(&self) -> Result<String> {
        let (_artifact, msg) = self.board.build(&self.root)?;
        Ok(msg)
    }

    fn cmd_run_sim(&mut self) -> Result<String> {
        if let Some(s) = &mut self.running {
            if s.is_alive() { bail!("simulator already running (use `stop` first)"); }
        }
        let ext = self.board.artifact_ext();
        let artifact = self.root.join("build")
            .join(format!("{}.{}", self.project.project.name, ext));
        if !artifact.exists() {
            bail!("artifact not found at {} — run `build` first", artifact.display());
        }
        let mut sim = self.board.spawn_sim(&self.root, &artifact, false)?;
        crate::sim::configure_peripherals(&mut sim, &self.project, self.board.spec())?;
        self.running = Some(sim);
        Ok(format!("✓ simulator started ({})", self.board.board_name()))
    }

    /// `adc <A0..A5|channel> <0..1023>` — 向运行中的仿真注入 ADC 值。
    fn cmd_adc(&mut self, args: &[&str]) -> Result<String> {
        let (Some(ch_arg), Some(val_arg)) = (args.first(), args.get(1)) else {
            bail!("usage: adc <A0..A5|channel> <0..1023>");
        };
        let spec = self.board.spec();
        let channel = if let Some(rest) = ch_arg.strip_prefix(['A', 'a']) {
            let a_pin: u8 = rest
                .parse()
                .map_err(|_| anyhow!("invalid analog pin: {}", ch_arg))?;
            spec.adc_channel_for(a_pin).ok_or_else(|| {
                anyhow!("{} has no ADC channel on {}", ch_arg.to_uppercase(), spec.board_id)
            })?
        } else {
            ch_arg
                .parse()
                .map_err(|_| anyhow!("invalid channel: {}", ch_arg))?
        };
        let value: u16 = val_arg
            .parse()
            .map_err(|_| anyhow!("invalid value: {} (expect 0..1023)", val_arg))?;
        let Some(sim) = self.running.as_mut() else {
            bail!("simulator not running — try `run`");
        };
        sim.set_adc(channel, value)?;
        Ok(format!("✓ adc ch{} = {}", channel, value.min(1023)))
    }

    /// `dist <cm>` — 设定超声波距离(2..400cm)。
    fn cmd_dist(&mut self, args: &[&str]) -> Result<String> {
        let Some(cm_arg) = args.first() else {
            bail!("usage: dist <2..400>  (cm)");
        };
        let cm: u16 = cm_arg
            .parse()
            .map_err(|_| anyhow!("invalid distance: {} (expect 2..400 cm)", cm_arg))?;
        let Some(sim) = self.running.as_mut() else {
            bail!("simulator not running — try `run`");
        };
        sim.set_distance(cm)?;
        Ok(format!("✓ dist = {}cm", cm.clamp(2, 400)))
    }

    /// `env <temp> <hum>` — 设定 DHT11 温湿度(0..50°C / 20..90%)。
    fn cmd_env(&mut self, args: &[&str]) -> Result<String> {
        let (Some(t_arg), Some(h_arg)) = (args.first(), args.get(1)) else {
            bail!("usage: env <temp 0..50> <hum 20..90>");
        };
        let temp: u8 = t_arg
            .parse()
            .map_err(|_| anyhow!("invalid temperature: {} (expect 0..50)", t_arg))?;
        let hum: u8 = h_arg
            .parse()
            .map_err(|_| anyhow!("invalid humidity: {} (expect 20..90)", h_arg))?;
        let Some(sim) = self.running.as_mut() else {
            bail!("simulator not running — try `run`");
        };
        sim.set_env(temp, hum)?;
        Ok(format!("✓ env = {}°C {}%", temp.min(50), hum.clamp(20, 90)))
    }

    /// `ir <hex32>` — 发送一帧 NEC 红外码(如 `ir 20DF10EF`)。
    fn cmd_ir(&mut self, args: &[&str]) -> Result<String> {
        let Some(code_arg) = args.first() else {
            bail!("usage: ir <hex32>  (e.g. ir 20DF10EF)");
        };
        let hex = code_arg.trim_start_matches("0x").trim_start_matches("0X");
        let code = u32::from_str_radix(hex, 16)
            .map_err(|_| anyhow!("invalid NEC code: {} (expect 32-bit hex)", code_arg))?;
        let Some(sim) = self.running.as_mut() else {
            bail!("simulator not running — try `run`");
        };
        sim.send_ir(code)?;
        Ok(format!("✓ ir {:08X}", code))
    }

    /// `send <text>` — 把 <text> 注入固件串口 RX(整行剩余部分,保留空格)。
    /// 例:`send hello` → 固件 Serial.read() 依次读到 h e l l o。
    fn cmd_send(&mut self, line: &str) -> Result<String> {
        // 取 "send " 之后的原始文本(保留空格,不做 token 化)
        let text = line.strip_prefix("send").map(|s| s.trim_start()).unwrap_or("");
        if text.is_empty() {
            bail!("usage: send <text>");
        }
        let Some(sim) = self.running.as_mut() else {
            bail!("simulator not running — try `run`");
        };
        sim.send_serial(text)?;
        Ok(format!("✓ sent {} byte(s)", text.len()))
    }

    fn cmd_stop(&mut self) -> Result<String> {
        if let Some(sim) = self.running.take() {
            sim.stop();
            Ok("✓ simulator stopped".to_string())
        } else {
            Ok("(no simulator running)".to_string())
        }
    }

    fn cmd_sleep(&self, args: &[&str]) -> Result<String> {
        let ms: u64 = args.first().copied().unwrap_or("1000").parse()
            .map_err(|_| anyhow!("usage: sleep <milliseconds>"))?;
        std::thread::sleep(std::time::Duration::from_millis(ms));
        Ok(String::new())
    }

    fn save_project(&self) -> Result<()> {
        self.project.save(&self.root.join("moxin.toml"))
    }
}

fn help_text() -> String {
    String::from(
        "moxin shell commands:
  board info                       show board details
  add <type> [args] --id <id>      add component:
        led [color]                  LED (default red)
        button                       push button
        resistor <ohms>              resistor (e.g. 10k, 4.7k, 470)
        buzzer                       piezo buzzer
        pot [max_ohms]               potentiometer (default 10k)
        ldr                          photoresistor (ambient light)
        rgb                          RGB LED (wire r/g/b terminals)
        servo                        SG90 servo (50Hz PWM)
        motor                        DC motor via L298N (ena/in1/in2)
        dht11                        DHT11 temp/humidity sensor
        ir                           IR receiver (NEC, VS1838)
        7seg                         seven-segment display
        breadboard                   breadboard
        dupont [color]               dupont jumper wire
  wire <from> -> <to>              add wire (e.g. pin13 -> led1.anode)
  show project                     dump project state
  show                             render runtime ASCII frame
  edit                             open src/main.ino in $EDITOR
  build                            compile via arduino-cli
  run                              start simavr simulator
  adc <A0..A5|ch> <0..1023>        inject ADC value (potentiometer knob)
  dist <2..400>                    set ultrasonic distance in cm
  env <temp> <hum>                 set DHT11 temperature/humidity
  ir <hex32>                       send NEC infrared frame
  send <text>                      inject text into firmware serial RX
  stop                             stop simulator
  exit | quit                      leave shell",
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn make_shell(tmp: &TempDir) -> Shell {
        let project = Project::new_blink("test");
        project.save(&tmp.path().join("moxin.toml")).expect("save project");
        let board = board_from_str(&project.project.board).expect("board_from_str");
        Shell { root: tmp.path().to_path_buf(), project, running: None, board }
    }

    #[test]
    fn test_dispatch_add_led_returns_success() {
        let tmp = TempDir::new().expect("tempdir");
        let mut shell = make_shell(&tmp);
        let msg = shell.dispatch("add led red --id led1").expect("add led should succeed");
        assert!(msg.contains("✓ added led1"), "got: {}", msg);
    }

    #[test]
    fn test_dispatch_add_without_id_returns_error() {
        let tmp = TempDir::new().expect("tempdir");
        let mut shell = make_shell(&tmp);
        let res = shell.dispatch("add led");
        assert!(res.is_err(), "missing --id should error, got: {:?}", res);
    }

    #[test]
    fn test_dispatch_wire_after_add_returns_success() {
        let tmp = TempDir::new().expect("tempdir");
        let mut shell = make_shell(&tmp);
        shell.dispatch("add led red --id led1").expect("add led prerequisite");
        let msg = shell.dispatch("wire pin13 -> led1.a").expect("wire should succeed");
        assert!(msg.contains("✓ wired"), "got: {}", msg);
    }

    #[test]
    fn wire_invalid_pin_returns_error() {
        let tmp = TempDir::new().expect("tempdir");
        let mut shell = make_shell(&tmp);
        shell.dispatch("add led red --id led1").unwrap();
        // D99 is rejected by PinRef::parse (parser enforces D0..D13 range)
        let res = shell.dispatch("wire D99 -> led1.a");
        assert!(res.is_err());
    }

    #[test]
    fn wire_unknown_port_pin_returns_error() {
        let tmp = TempDir::new().expect("tempdir");
        let mut shell = make_shell(&tmp);
        shell.dispatch("add led red --id led1").unwrap();
        // PZ99 parses as BoardPort but is not in arduino-uno spec
        let res = shell.dispatch("wire PZ99 -> led1.a");
        assert!(res.is_err());
        assert!(res.unwrap_err().to_string().contains("unknown pin"));
    }

    #[test]
    fn wire_valid_d7_succeeds() {
        let tmp = TempDir::new().expect("tempdir");
        let mut shell = make_shell(&tmp);
        shell.dispatch("add led red --id led1").unwrap();
        let res = shell.dispatch("wire D7 -> led1.a");
        assert!(res.is_ok(), "D7 should be valid, got: {:?}", res);
    }

    #[test]
    fn parse_resistance_plain_ohms() {
        assert_eq!(crate::components::util::parse_resistance("470").unwrap(), 470);
    }

    #[test]
    fn parse_resistance_kilo() {
        assert_eq!(crate::components::util::parse_resistance("10k").unwrap(), 10_000);
    }

    #[test]
    fn parse_resistance_fractional_kilo() {
        assert_eq!(crate::components::util::parse_resistance("4.7k").unwrap(), 4700);
    }

    #[test]
    fn parse_resistance_mega() {
        assert_eq!(crate::components::util::parse_resistance("1m").unwrap(), 1_000_000);
    }

    #[test]
    fn parse_resistance_zero_is_error() {
        assert!(crate::components::util::parse_resistance("0").is_err());
    }

    #[test]
    fn parse_resistance_invalid_is_error() {
        assert!(crate::components::util::parse_resistance("abc").is_err());
    }

    #[test]
    fn add_resistor_succeeds() {
        let tmp = TempDir::new().expect("tempdir");
        let mut shell = make_shell(&tmp);
        let msg = shell.dispatch("add resistor 10k --id r1").unwrap();
        assert!(msg.contains("✓ added r1"), "got: {}", msg);
        assert_eq!(shell.project.components[0].ohms, Some(10_000));
    }

    #[test]
    fn add_resistor_missing_ohms_is_error() {
        let tmp = TempDir::new().expect("tempdir");
        let mut shell = make_shell(&tmp);
        assert!(shell.dispatch("add resistor --id r1").is_err());
    }

    #[test]
    fn adc_without_running_sim_is_error() {
        let tmp = TempDir::new().expect("tempdir");
        let mut shell = make_shell(&tmp);
        let res = shell.dispatch("adc A0 512");
        assert!(res.is_err());
        assert!(res.unwrap_err().to_string().contains("not running"));
    }

    #[test]
    fn adc_invalid_pin_is_error() {
        let tmp = TempDir::new().expect("tempdir");
        let mut shell = make_shell(&tmp);
        // A9 不存在:通道解析先于"仿真未运行"检查报错
        let res = shell.dispatch("adc A9 512");
        assert!(res.is_err());
        assert!(res.unwrap_err().to_string().contains("no ADC channel"));
    }

    #[test]
    fn adc_usage_error_without_args() {
        let tmp = TempDir::new().expect("tempdir");
        let mut shell = make_shell(&tmp);
        let res = shell.dispatch("adc");
        assert!(res.is_err());
        assert!(res.unwrap_err().to_string().contains("usage"));
    }

    #[test]
    fn dist_without_running_sim_is_error() {
        let tmp = TempDir::new().expect("tempdir");
        let mut shell = make_shell(&tmp);
        let res = shell.dispatch("dist 100");
        assert!(res.is_err());
        assert!(res.unwrap_err().to_string().contains("not running"));
    }

    #[test]
    fn dist_invalid_arg_is_error() {
        let tmp = TempDir::new().expect("tempdir");
        let mut shell = make_shell(&tmp);
        assert!(shell.dispatch("dist abc").is_err());
        assert!(shell.dispatch("dist").is_err());
    }

    #[test]
    fn send_without_running_sim_is_error() {
        let tmp = TempDir::new().expect("tempdir");
        let mut shell = make_shell(&tmp);
        let res = shell.dispatch("send hello");
        assert!(res.is_err());
        assert!(res.unwrap_err().to_string().contains("not running"));
    }

    #[test]
    fn send_without_text_is_usage_error() {
        let tmp = TempDir::new().expect("tempdir");
        let mut shell = make_shell(&tmp);
        let res = shell.dispatch("send");
        assert!(res.is_err());
        assert!(res.unwrap_err().to_string().contains("usage"));
    }

    #[test]
    fn add_ultrasonic_via_alias() {
        let tmp = TempDir::new().expect("tempdir");
        let mut shell = make_shell(&tmp);
        let msg = shell.dispatch("add sr04 --id us1").unwrap();
        assert!(msg.contains("✓ added us1"), "got: {}", msg);
        assert_eq!(shell.project.components[0].kind, "ultrasonic");
    }

    #[test]
    fn add_buzzer_succeeds() {
        let tmp = TempDir::new().expect("tempdir");
        let mut shell = make_shell(&tmp);
        let msg = shell.dispatch("add buzzer --id bz1").unwrap();
        assert!(msg.contains("✓ added bz1"), "got: {}", msg);
    }

    #[test]
    fn add_potentiometer_default_max() {
        let tmp = TempDir::new().expect("tempdir");
        let mut shell = make_shell(&tmp);
        shell.dispatch("add pot --id p1").unwrap();
        assert_eq!(shell.project.components[0].max_ohms, Some(10_000));
    }

    #[test]
    fn add_potentiometer_custom_max() {
        let tmp = TempDir::new().expect("tempdir");
        let mut shell = make_shell(&tmp);
        shell.dispatch("add pot 100k --id p1").unwrap();
        assert_eq!(shell.project.components[0].max_ohms, Some(100_000));
    }

    #[test]
    fn add_seven_segment_succeeds() {
        let tmp = TempDir::new().expect("tempdir");
        let mut shell = make_shell(&tmp);
        let msg = shell.dispatch("add 7seg --id seg1").unwrap();
        assert!(msg.contains("✓ added seg1"), "got: {}", msg);
    }

    #[test]
    fn add_breadboard_succeeds() {
        let tmp = TempDir::new().expect("tempdir");
        let mut shell = make_shell(&tmp);
        let msg = shell.dispatch("add breadboard --id bb1").unwrap();
        assert!(msg.contains("✓ added bb1"), "got: {}", msg);
    }

    #[test]
    fn add_dupont_with_color() {
        let tmp = TempDir::new().expect("tempdir");
        let mut shell = make_shell(&tmp);
        shell.dispatch("add dupont red --id w1").unwrap();
        assert_eq!(shell.project.components[0].wire_color, Some("red".into()));
    }

    #[test]
    fn add_dupont_without_color() {
        let tmp = TempDir::new().expect("tempdir");
        let mut shell = make_shell(&tmp);
        shell.dispatch("add dupont --id w1").unwrap();
        assert_eq!(shell.project.components[0].wire_color, None);
    }
}
