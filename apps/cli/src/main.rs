use peachy_document::Document;
use peachy_engine::{Command, Session, evaluate};
use serde_json::{Value, json};
use std::{env, fs, path::Path, process};

fn main() {
    if let Err(e) = run() {
        eprintln!("{e}");
        process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let args: Vec<_> = env::args().skip(1).collect();
    let usage = "Peachy Animator CLI\n  validate PROJECT.peachy\n  inspect SPINE.json\n  import SPINE.json OUTPUT.peachy\n  export PROJECT.peachy OUTPUT.json\n  pose PROJECT.peachy ANIMATION SECONDS\n  apply PROJECT.peachy COMMANDS.json OUTPUT.peachy\n\nAll mutation commands write a new output. Artist source files are never edited.";
    if args.is_empty() || args[0] == "--help" {
        println!("{usage}");
        return Ok(());
    }
    let input = args.get(1).ok_or(usage)?;
    let contents = fs::read_to_string(input).map_err(|e| e.to_string())?;
    match args[0].as_str() {
        "inspect" => {
            let v: Value = serde_json::from_str(&contents).map_err(|e| e.to_string())?;
            println!(
                "{}",
                serde_json::to_string_pretty(&peachy_spine::inspect(&v)).unwrap()
            );
        }
        "import" => {
            let v: Value = serde_json::from_str(&contents).map_err(|e| e.to_string())?;
            let doc = peachy_spine::import_json(
                &v,
                Path::new(input)
                    .file_stem()
                    .and_then(|p| p.to_str())
                    .unwrap_or("Imported rig"),
            )?;
            let output = args.get(2).ok_or(usage)?;
            require_extension(output, "peachy")?;
            write_new(
                output,
                &serde_json::to_string_pretty(&doc).map_err(|e| e.to_string())?,
            )?;
        }
        "validate" => {
            let doc: Document = serde_json::from_str(&contents).map_err(|e| e.to_string())?;
            doc.validate()?;
            println!(
                "{}",
                json!({"valid":true,"bones":doc.bones.len(),"animations":doc.animations.len()})
            );
        }
        "export" => {
            let doc: Document = serde_json::from_str(&contents).map_err(|e| e.to_string())?;
            let output = args.get(2).ok_or(usage)?;
            require_extension(output, "json")?;
            write_new(
                output,
                &serde_json::to_string_pretty(&peachy_spine::export_json(&doc)?)
                    .map_err(|e| e.to_string())?,
            )?;
        }
        "pose" => {
            let doc: Document = serde_json::from_str(&contents).map_err(|e| e.to_string())?;
            doc.validate()?;
            let name = args.get(2).ok_or(usage)?;
            let animation = doc
                .animations
                .iter()
                .position(|a| &a.name == name)
                .ok_or("Unknown animation")?;
            let time: f32 = args
                .get(3)
                .ok_or(usage)?
                .parse()
                .map_err(|_| "Invalid time")?;
            if !time.is_finite() {
                return Err("Time must be finite".into());
            }
            let pose = evaluate(&doc, Some(animation), time);
            println!("{}",serde_json::to_string_pretty(&json!({"time":time,"bones":doc.bones.iter().zip(pose).map(|(b,p)|json!({"name":b.name,"world":p})).collect::<Vec<_>>()})).unwrap());
        }
        "apply" => {
            let doc: Document = serde_json::from_str(&contents).map_err(|e| e.to_string())?;
            let commands: Vec<Command> = serde_json::from_str(
                &fs::read_to_string(args.get(2).ok_or(usage)?).map_err(|e| e.to_string())?,
            )
            .map_err(|e| e.to_string())?;
            let mut session = Session::new(doc)?;
            session.dispatch(Command::Batch { commands })?;
            let output = args.get(3).ok_or(usage)?;
            require_extension(output, "peachy")?;
            write_new(
                output,
                &serde_json::to_string_pretty(session.document()).unwrap(),
            )?;
        }
        _ => return Err(usage.into()),
    }
    Ok(())
}

fn require_extension(path: &str, extension: &str) -> Result<(), String> {
    if Path::new(path).extension().and_then(|x| x.to_str()) != Some(extension) {
        return Err(format!("Output must have .{extension} extension"));
    }
    Ok(())
}
fn write_new(path: &str, text: &str) -> Result<(), String> {
    use std::io::Write;
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|e| format!("Refusing to overwrite output {path}: {e}"))?;
    file.write_all(text.as_bytes())
        .and_then(|()| file.write_all(b"\n"))
        .map_err(|e| e.to_string())
}
