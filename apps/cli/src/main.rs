use peachy_engine::{Command, apply_command, checked_evaluate_scene};
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
    let usage = "Peachy Animator CLI\n  validate PROJECT.peachy\n  inspect SPINE.json\n  import SPINE.json OUTPUT.peachy\n  export PROJECT.peachy OUTPUT.json\n  bundle PROJECT.peachy PARENT_FOLDER\n  pose PROJECT.peachy ANIMATION SECONDS [SKIN]\n  apply PROJECT.peachy COMMANDS.json OUTPUT.peachy\n\nAll mutation commands write a new output. Artist source files are never edited.";
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
            peachy_io::create_project(Path::new(output), &doc)?;
        }
        "validate" => {
            let doc = peachy_io::decode_project(&contents)?;
            println!(
                "{}",
                json!({"valid":true,"bones":doc.bones.len(),"animations":doc.animations.len()})
            );
        }
        "export" => {
            let doc = peachy_io::decode_project(&contents)?;
            let output = args.get(2).ok_or(usage)?;
            peachy_io::export_json(Path::new(output), &doc)?;
        }
        "bundle" => {
            let doc = peachy_io::decode_project(&contents)?;
            let parent = Path::new(args.get(2).ok_or(usage)?);
            let folder = peachy_io::export_bundle(
                parent,
                &doc,
                Path::new(input)
                    .parent()
                    .filter(|p| !p.as_os_str().is_empty())
                    .unwrap_or_else(|| Path::new(".")),
            )?;
            println!("{}", folder.display());
        }
        "pose" => {
            let doc = peachy_io::decode_project(&contents)?;
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
            let skin = args.get(4).map(String::as_str).unwrap_or("default");
            if !doc.skins.is_empty() && !doc.skins.contains_key(skin) {
                return Err("Unknown skin".into());
            }
            let scene = checked_evaluate_scene(&doc, Some(animation), time, skin, None)?;
            println!("{}",serde_json::to_string_pretty(&json!({"time":time,"bones":doc.bones.iter().zip(scene.world).map(|(b,p)|json!({"name":b.name,"world":p})).collect::<Vec<_>>(),"attachments":scene.attachments})).unwrap());
        }
        "apply" => {
            let doc = peachy_io::decode_project(&contents)?;
            let commands: Vec<Command> = serde_json::from_str(
                &fs::read_to_string(args.get(2).ok_or(usage)?).map_err(|e| e.to_string())?,
            )
            .map_err(|e| e.to_string())?;
            let edited = apply_command(&doc, Command::Batch { commands })?;
            let output = args.get(3).ok_or(usage)?;
            peachy_io::create_project(Path::new(output), &edited)?;
        }
        _ => return Err(usage.into()),
    }
    Ok(())
}
