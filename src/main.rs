use bytes::Bytes;
use clap::Parser;
use crc::{Crc, CRC_32_CKSUM};
use eolib::{
    data::{decode_number, encode_number, EoReader, EoSerialize, EoWriter},
    protocol::r#pub::server::{
        DropFile, DropNpcRecord, DropRecord, InnFile, InnQuestionRecord, InnRecord,
        ShopCraftIngredientRecord, ShopCraftRecord, ShopFile, ShopRecord, ShopTradeRecord,
        SkillMasterFile, SkillMasterRecord, SkillMasterSkillRecord, TalkFile, TalkMessageRecord,
        TalkRecord,
    },
    protocol::r#pub::{
        Ecf, EcfRecord, Eif, EifRecord, Element, Enf, EnfRecord, Esf, EsfRecord, ItemSize,
        ItemSpecial, ItemSubtype, ItemType, NpcType, SkillNature, SkillTargetRestrict,
        SkillTargetType, SkillType,
    },
};
use glob::glob;
use serde_json::{json, Value};
use std::{
    fs::File,
    io::{Read, Write},
    path::Path,
};

const CRC32: Crc<u32> = Crc::<u32>::new(&CRC_32_CKSUM);

/// A little tool to convert EO data files to JSON
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// path to directory containing pub files
    #[arg(short, long, default_value = "./pub")]
    pubs: String,

    /// path to directory containing JSON files
    #[arg(short, long, default_value = "./pub_json")]
    json: String,

    /// What type of server data files are you converting
    #[arg(short, long, default_value = "original")]
    server: ServerData,

    /// reverse mode: convert JSON files back to binary pub files
    #[arg(short, long, default_value_t = false)]
    reverse: bool,
}

#[derive(clap::ValueEnum, Clone, Debug, PartialEq, Eq)]
enum ServerData {
    Original,
    EOSERV,
    PubStudio,
}

fn main() {
    let args = Args::parse();

    println!(
        "             _    ___  _
            | |  |__ \\(_)
 _ __  _   _| |__   ) |_ ___  ___  _ __
| '_ \\| | | | '_ \\ / /| / __|/ _ \\| '_ \\
| |_) | |_| | |_) / /_| \\__ \\ (_) | | | |
| .__/ \\__,_|_.__/____| |___/\\___/|_| |_|
| |                  _/ |
|_|                 |__/\n"
    );

    if args.reverse {
        run_json2pub(&args);
    } else {
        run_pub2json(&args);
    }
}

fn run_pub2json(args: &Args) {
    let class_file = load_class_file(&args.pubs);
    let drop_file = match load_drop_file(&args.pubs) {
        Ok(f) => f,
        Err(_) => {
            println!("Could not load drop file, using default");
            DropFile::default()
        }
    };
    let inn_file = match load_inn_file(&args.pubs) {
        Ok(f) => f,
        Err(_) => {
            println!("Could not load inn file, using default");
            InnFile::default()
        }
    };
    let item_file = load_item_file(&args.pubs);
    let npc_file = load_npc_file(&args.pubs);
    let shop_file = match load_shop_file(&args.pubs) {
        Ok(f) => f,
        Err(_) => {
            println!("Could not load shop file, using default");
            ShopFile::default()
        }
    };
    let skill_master_file = match load_skill_master_file(&args.pubs) {
        Ok(f) => f,
        Err(_) => {
            println!("Could not load skill master file, using default");
            SkillMasterFile::default()
        }
    };
    let spell_file = load_spell_file(&args.pubs);
    let talk_file = match load_talk_file(&args.pubs) {
        Ok(f) => f,
        Err(_) => {
            println!("Could not load talk file, using default");
            TalkFile::default()
        }
    };

    match class_file {
        Ok(f) => {
            let _ = generate_class_json(&f, &args.json);
        }
        Err(e) => println!("Could not load class file: {}", e),
    }
    match spell_file {
        Ok(f) => {
            let _ = generate_spell_json(&f, &args.json);
        }
        Err(e) => println!("Could not load spell file: {}", e),
    }
    match item_file {
        Ok(f) => {
            let _ = generate_item_json(&f, &args.json);
        }
        Err(e) => println!("Could not load item file: {}", e),
    }
    match npc_file {
        Ok(f) => {
            let _ = generate_npc_json(&f, &drop_file, &talk_file, &args.json);
        }
        Err(e) => println!("Could not load npc file: {}", e),
    }
    let _ = generate_shop_json(&shop_file, &args.json);
    let _ = generate_inn_json(&inn_file, &args.json);
    let _ = generate_skill_master_json(&skill_master_file, &args.json);
}

fn run_json2pub(args: &Args) {
    std::fs::create_dir_all(&args.pubs).unwrap();

    match json2pub_classes(&args.json, &args.pubs) {
        Ok(_) => {}
        Err(e) => println!("Could not generate class file: {}", e),
    }
    match json2pub_spells(&args.json, &args.pubs) {
        Ok(_) => {}
        Err(e) => println!("Could not generate spell file: {}", e),
    }
    match json2pub_items(&args.json, &args.pubs) {
        Ok(_) => {}
        Err(e) => println!("Could not generate item file: {}", e),
    }
    match json2pub_npcs(&args.json, &args.pubs) {
        Ok(_) => {}
        Err(e) => println!("Could not generate npc/drop/talk files: {}", e),
    }
    match json2pub_shops(&args.json, &args.pubs) {
        Ok(_) => {}
        Err(e) => println!("Could not generate shop file: {}", e),
    }
    match json2pub_inns(&args.json, &args.pubs) {
        Ok(_) => {}
        Err(e) => println!("Could not generate inn file: {}", e),
    }
    match json2pub_skill_masters(&args.json, &args.pubs) {
        Ok(_) => {}
        Err(e) => println!("Could not generate skill master file: {}", e),
    }
}

// ─── pub → JSON ──────────────────────────────────────────────────────────────

fn generate_class_json(class_file: &Ecf, path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let dir = Path::new(path).join("classes");
    if dir.exists() {
        std::fs::remove_dir_all(dir.clone())?;
    }
    std::fs::create_dir_all(dir.clone())?;

    let mut id = 1;
    for class in &class_file.classes {
        if class.name == "eof" {
            continue;
        }
        let json = json!({
            "name": class.name,
            "parent_type": class.parent_type,
            "stat_group": class.stat_group,
            "str": class.str,
            "intl": class.intl,
            "wis": class.wis,
            "agi": class.agi,
            "con": class.con,
            "cha": class.cha,
        });
        std::fs::write(
            dir.join(format!("{:0>4}.json", id)),
            serde_json::to_string_pretty(&json).unwrap(),
        )?;
        id += 1;
    }
    println!("✨ Generated {} class files", id - 1);
    Ok(())
}

fn generate_spell_json(spell_file: &Esf, path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let dir = Path::new(path).join("spells");
    if dir.exists() {
        std::fs::remove_dir_all(dir.clone())?;
    }
    std::fs::create_dir_all(dir.clone())?;

    let mut id = 1;
    for spell in &spell_file.skills {
        if spell.name == "eof" {
            continue;
        }
        let json = json!({
            "name": spell.name,
            "chant": spell.chant,
            "icon_id": spell.icon_id,
            "graphic_id": spell.graphic_id,
            "tp_cost": spell.tp_cost,
            "sp_cost": spell.sp_cost,
            "cast_time": spell.cast_time,
            "nature": i32::from(spell.nature),
            "type": i32::from(spell.r#type),
            "element": i32::from(spell.element),
            "element_power": spell.element_power,
            "target_restrict": i32::from(spell.target_restrict),
            "target_type": i32::from(spell.target_type),
            "target_time": spell.target_time,
            "max_skill_level": spell.max_skill_level,
            "min_damage": spell.min_damage,
            "max_damage": spell.max_damage,
            "accuracy": spell.accuracy,
            "evade": spell.evade,
            "armor": spell.armor,
            "return_damage": spell.return_damage,
            "hp_heal": spell.hp_heal,
            "tp_heal": spell.tp_heal,
            "sp_heal": spell.sp_heal,
            "str": spell.str,
            "intl": spell.intl,
            "wis": spell.wis,
            "agi": spell.agi,
            "con": spell.con,
            "cha": spell.cha,
        });
        std::fs::write(
            dir.join(format!("{:0>4}.json", id)),
            serde_json::to_string_pretty(&json).unwrap(),
        )?;
        id += 1;
    }
    println!("✨ Generated {} spell files", id - 1);
    Ok(())
}

fn generate_item_json(item_file: &Eif, path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let dir = Path::new(path).join("items");
    if dir.exists() {
        std::fs::remove_dir_all(dir.clone())?;
    }
    std::fs::create_dir_all(dir.clone())?;

    let mut id = 1;
    for item in &item_file.items {
        if item.name == "eof" {
            continue;
        }
        let json = json!({
            "name": item.name,
            "graphic_id": item.graphic_id,
            "type": i32::from(item.r#type),
            "subtype": i32::from(item.subtype),
            "special": i32::from(item.special),
            "hp": item.hp,
            "tp": item.tp,
            "min_damage": item.min_damage,
            "max_damage": item.max_damage,
            "accuracy": item.accuracy,
            "evade": item.evade,
            "armor": item.armor,
            "return_damage": item.return_damage,
            "str": item.str,
            "intl": item.intl,
            "wis": item.wis,
            "agi": item.agi,
            "con": item.con,
            "cha": item.cha,
            "light_resistance": item.light_resistance,
            "dark_resistance": item.dark_resistance,
            "earth_resistance": item.earth_resistance,
            "air_resistance": item.air_resistance,
            "water_resistance": item.water_resistance,
            "fire_resistance": item.fire_resistance,
            "spec1": item.spec1,
            "spec2": item.spec2,
            "spec3": item.spec3,
            "level_requirement": item.level_requirement,
            "class_requirement": item.class_requirement,
            "str_requirement": item.str_requirement,
            "int_requirement": item.int_requirement,
            "wis_requirement": item.wis_requirement,
            "agi_requirement": item.agi_requirement,
            "con_requirement": item.con_requirement,
            "cha_requirement": item.cha_requirement,
            "element": i32::from(item.element),
            "element_damage": item.element_damage,
            "weight": item.weight,
            "size": i32::from(item.size),
        });
        std::fs::write(
            dir.join(format!("{:0>4}.json", id)),
            serde_json::to_string_pretty(&json).unwrap(),
        )?;
        id += 1;
    }
    println!("✨ Generated {} item files", id - 1);
    Ok(())
}

fn generate_npc_json(
    npc_file: &Enf,
    drop_file: &DropFile,
    talk_file: &TalkFile,
    path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let dir = Path::new(path).join("npcs");
    if dir.exists() {
        std::fs::remove_dir_all(dir.clone())?;
    }
    std::fs::create_dir_all(dir.clone())?;

    let mut id = 1;
    for npc in &npc_file.npcs {
        if npc.name == "eof" {
            continue;
        }

        let drop_record = drop_file
            .npcs
            .iter()
            .find(|n| n.npc_id == id)
            .cloned()
            .unwrap_or_default();

        let talk_record = talk_file
            .npcs
            .iter()
            .find(|n| n.npc_id == id)
            .cloned()
            .unwrap_or_default();

        let json = json!({
            "name": npc.name,
            "graphic_id": npc.graphic_id,
            "race": npc.race,
            "boss": npc.boss,
            "child": npc.child,
            "type": i32::from(npc.r#type),
            "behavior_id": npc.behavior_id,
            "hp": npc.hp,
            "tp": npc.tp,
            "min_damage": npc.min_damage,
            "max_damage": npc.max_damage,
            "accuracy": npc.accuracy,
            "evade": npc.evade,
            "armor": npc.armor,
            "return_damage": npc.return_damage,
            "element": i32::from(npc.element),
            "element_damage": npc.element_damage,
            "element_weakness": i32::from(npc.element_weakness),
            "element_weakness_damage": npc.element_weakness_damage,
            "level": npc.level,
            "experience": npc.experience,
            "drops": drop_record.drops.iter().map(|d| json!({
                "item_id": d.item_id,
                "min_amount": d.min_amount,
                "max_amount": d.max_amount,
                "rate": d.rate,
            })).collect::<Vec<_>>(),
            "talk_rate": talk_record.rate,
            "talk_messages": talk_record.messages.iter().map(|m| &m.message).collect::<Vec<_>>(),
        });
        std::fs::write(
            dir.join(format!("{:0>4}.json", id)),
            serde_json::to_string_pretty(&json).unwrap(),
        )?;
        id += 1;
    }
    println!("✨ Generated {} npc files", id - 1);
    Ok(())
}

fn generate_shop_json(shop_file: &ShopFile, path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let dir = Path::new(path).join("shops");
    if dir.exists() {
        std::fs::remove_dir_all(dir.clone())?;
    }
    std::fs::create_dir_all(dir.clone())?;

    let mut id = 1;
    for shop in &shop_file.shops {
        if shop.name == "eof" {
            continue;
        }
        let json = json!({
            "behavior_id": shop.behavior_id,
            "name": shop.name,
            "min_level": shop.min_level,
            "max_level": shop.max_level,
            "class_requirement": shop.class_requirement,
            "trades": shop.trades.iter().map(|t| json!({
                "item_id": t.item_id,
                "buy_price": t.buy_price,
                "sell_price": t.sell_price,
                "max_amount": t.max_amount,
            })).collect::<Vec<_>>(),
            "crafts": shop.crafts.iter().map(|c| json!({
                "item_id": c.item_id,
                "ingredients": c.ingredients.iter().map(|i| json!({
                    "item_id": i.item_id,
                    "amount": i.amount,
                })).collect::<Vec<_>>(),
            })).collect::<Vec<_>>(),
        });
        std::fs::write(
            dir.join(format!("{:0>4}.json", id)),
            serde_json::to_string_pretty(&json).unwrap(),
        )?;
        id += 1;
    }
    println!("✨ Generated {} shop files", id - 1);
    Ok(())
}

fn generate_inn_json(inn_file: &InnFile, path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let dir = Path::new(path).join("inns");
    if dir.exists() {
        std::fs::remove_dir_all(dir.clone())?;
    }
    std::fs::create_dir_all(dir.clone())?;

    let mut id = 1;
    for inn in &inn_file.inns {
        if inn.name == "eof" {
            continue;
        }
        let json = json!({
            "behavior_id": inn.behavior_id,
            "name": inn.name,
            "spawn_map": inn.spawn_map,
            "spawn_x": inn.spawn_x,
            "spawn_y": inn.spawn_y,
            "sleep_map": inn.sleep_map,
            "sleep_x": inn.sleep_x,
            "sleep_y": inn.sleep_y,
            "alternate_spawn_enabled": inn.alternate_spawn_enabled,
            "alternate_spawn_map": inn.alternate_spawn_map,
            "alternate_spawn_x": inn.alternate_spawn_x,
            "alternate_spawn_y": inn.alternate_spawn_y,
            "questions": inn.questions.iter().map(|q| json!({
                "question": q.question,
                "answer": q.answer,
            })).collect::<Vec<_>>(),
        });
        std::fs::write(
            dir.join(format!("{:0>4}.json", id)),
            serde_json::to_string_pretty(&json).unwrap(),
        )?;
        id += 1;
    }
    println!("✨ Generated {} inn files", id - 1);
    Ok(())
}

fn generate_skill_master_json(
    skill_master_file: &SkillMasterFile,
    path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let dir = Path::new(path).join("skill_masters");
    if dir.exists() {
        std::fs::remove_dir_all(dir.clone())?;
    }
    std::fs::create_dir_all(dir.clone())?;

    let mut id = 1;
    for sm in &skill_master_file.skill_masters {
        if sm.name == "eof" {
            continue;
        }
        let json = json!({
            "behavior_id": sm.behavior_id,
            "name": sm.name,
            "min_level": sm.min_level,
            "max_level": sm.max_level,
            "class_requirement": sm.class_requirement,
            "skills": sm.skills.iter().map(|s| json!({
                "id": s.skill_id,
                "level_requirement": s.level_requirement,
                "class_requirement": s.class_requirement,
                "price": s.price,
                "skill_requirements": s.skill_requirements,
                "str_requirement": s.str_requirement,
                "int_requirement": s.int_requirement,
                "wis_requirement": s.wis_requirement,
                "agi_requirement": s.agi_requirement,
                "con_requirement": s.con_requirement,
                "cha_requirement": s.cha_requirement,
            })).collect::<Vec<_>>(),
        });
        std::fs::write(
            dir.join(format!("{:0>4}.json", id)),
            serde_json::to_string_pretty(&json).unwrap(),
        )?;
        id += 1;
    }
    println!("✨ Generated {} skill master files", id - 1);
    Ok(())
}

// ─── JSON → pub ──────────────────────────────────────────────────────────────

fn save_pub_file<T: EoSerialize>(file: &T, path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut writer = EoWriter::new();
    file.serialize(&mut writer).unwrap();
    let buf = writer.to_byte_array();
    let mut f = File::create(path)?;
    f.write_all(&buf)?;
    Ok(())
}

fn set_crc32_rid<T: EoSerialize>(file: &mut T) -> Vec<u8> {
    let mut writer = EoWriter::new();
    file.serialize(&mut writer).unwrap();
    let buf = writer.to_byte_array();
    let mut digest = CRC32.digest();
    digest.update(&buf[7..]);
    let checksum = digest.finalize();
    let encoded = encode_number(checksum as i32).unwrap();
    (encoded[0..4]).to_vec()
}

fn json2pub_classes(json_path: &str, pub_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut ecf = Ecf::default();
    let pattern = Path::new(json_path).join("classes/*.json");
    let mut entries: Vec<_> = glob(pattern.to_str().unwrap())?
        .filter_map(|e| e.ok())
        .collect();
    entries.sort();
    for entry in &entries {
        let v: Value = serde_json::from_str(&std::fs::read_to_string(entry)?)?;
        ecf.classes.push(EcfRecord {
            name: v["name"].as_str().unwrap_or_default().to_string(),
            parent_type: v["parent_type"].as_u64().unwrap_or(0) as i32,
            stat_group: v["stat_group"].as_u64().unwrap_or(0) as i32,
            str: v["str"].as_u64().unwrap_or(0) as i32,
            intl: v["intl"].as_u64().unwrap_or(0) as i32,
            wis: v["wis"].as_u64().unwrap_or(0) as i32,
            agi: v["agi"].as_u64().unwrap_or(0) as i32,
            con: v["con"].as_u64().unwrap_or(0) as i32,
            cha: v["cha"].as_u64().unwrap_or(0) as i32,
        });
    }
    ecf.classes.push(EcfRecord {
        name: "eof".to_string(),
        ..Default::default()
    });
    ecf.total_classes_count = ecf.classes.len() as i32;
    let encoded = set_crc32_rid(&mut ecf);
    ecf.rid = [
        decode_number(&encoded[0..=1]) as i32,
        decode_number(&encoded[2..=3]) as i32,
    ];
    save_pub_file(&ecf, &format!("{}/dat001.ecf", pub_path))?;
    println!("✨ Generated dat001.ecf ({} classes)", entries.len());
    Ok(())
}

fn json2pub_spells(json_path: &str, pub_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut esf = Esf::default();
    let pattern = Path::new(json_path).join("spells/*.json");
    let mut entries: Vec<_> = glob(pattern.to_str().unwrap())?
        .filter_map(|e| e.ok())
        .collect();
    entries.sort();
    for entry in &entries {
        let v: Value = serde_json::from_str(&std::fs::read_to_string(entry)?)?;
        esf.skills.push(EsfRecord {
            name: v["name"].as_str().unwrap_or_default().to_string(),
            chant: v["chant"].as_str().unwrap_or_default().to_string(),
            icon_id: v["icon_id"].as_u64().unwrap_or(0) as i32,
            graphic_id: v["graphic_id"].as_u64().unwrap_or(0) as i32,
            tp_cost: v["tp_cost"].as_u64().unwrap_or(0) as i32,
            sp_cost: v["sp_cost"].as_u64().unwrap_or(0) as i32,
            cast_time: v["cast_time"].as_u64().unwrap_or(0) as i32,
            nature: SkillNature::from(v["nature"].as_u64().unwrap_or(0) as i32),
            r#type: SkillType::from(v["type"].as_u64().unwrap_or(0) as i32),
            element: Element::from(v["element"].as_u64().unwrap_or(0) as i32),
            element_power: v["element_power"].as_u64().unwrap_or(0) as i32,
            target_restrict: SkillTargetRestrict::from(
                v["target_restrict"].as_u64().unwrap_or(0) as i32
            ),
            target_type: SkillTargetType::from(v["target_type"].as_u64().unwrap_or(0) as i32),
            target_time: v["target_time"].as_u64().unwrap_or(0) as i32,
            max_skill_level: v["max_skill_level"].as_u64().unwrap_or(0) as i32,
            min_damage: v["min_damage"].as_u64().unwrap_or(0) as i32,
            max_damage: v["max_damage"].as_u64().unwrap_or(0) as i32,
            accuracy: v["accuracy"].as_u64().unwrap_or(0) as i32,
            evade: v["evade"].as_u64().unwrap_or(0) as i32,
            armor: v["armor"].as_u64().unwrap_or(0) as i32,
            return_damage: v["return_damage"].as_u64().unwrap_or(0) as i32,
            hp_heal: v["hp_heal"].as_u64().unwrap_or(0) as i32,
            tp_heal: v["tp_heal"].as_u64().unwrap_or(0) as i32,
            sp_heal: v["sp_heal"].as_u64().unwrap_or(0) as i32,
            str: v["str"].as_u64().unwrap_or(0) as i32,
            intl: v["intl"].as_u64().unwrap_or(0) as i32,
            wis: v["wis"].as_u64().unwrap_or(0) as i32,
            agi: v["agi"].as_u64().unwrap_or(0) as i32,
            con: v["con"].as_u64().unwrap_or(0) as i32,
            cha: v["cha"].as_u64().unwrap_or(0) as i32,
        });
    }
    esf.skills.push(EsfRecord {
        name: "eof".to_string(),
        ..Default::default()
    });
    esf.total_skills_count = esf.skills.len() as i32;
    let encoded = set_crc32_rid(&mut esf);
    esf.rid = [
        decode_number(&encoded[0..=1]) as i32,
        decode_number(&encoded[2..=3]) as i32,
    ];
    save_pub_file(&esf, &format!("{}/dsl001.esf", pub_path))?;
    println!("✨ Generated dsl001.esf ({} spells)", entries.len());
    Ok(())
}

fn json2pub_items(json_path: &str, pub_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut eif = Eif::default();
    let pattern = Path::new(json_path).join("items/*.json");
    let mut entries: Vec<_> = glob(pattern.to_str().unwrap())?
        .filter_map(|e| e.ok())
        .collect();
    entries.sort();
    for entry in &entries {
        let v: Value = serde_json::from_str(&std::fs::read_to_string(entry)?)?;
        eif.items.push(EifRecord {
            name: v["name"].as_str().unwrap_or_default().to_string(),
            graphic_id: v["graphic_id"].as_u64().unwrap_or(0) as i32,
            r#type: ItemType::from(v["type"].as_u64().unwrap_or(0) as i32),
            subtype: ItemSubtype::from(v["subtype"].as_u64().unwrap_or(0) as i32),
            special: ItemSpecial::from(v["special"].as_u64().unwrap_or(0) as i32),
            hp: v["hp"].as_u64().unwrap_or(0) as i32,
            tp: v["tp"].as_u64().unwrap_or(0) as i32,
            min_damage: v["min_damage"].as_u64().unwrap_or(0) as i32,
            max_damage: v["max_damage"].as_u64().unwrap_or(0) as i32,
            accuracy: v["accuracy"].as_u64().unwrap_or(0) as i32,
            evade: v["evade"].as_u64().unwrap_or(0) as i32,
            armor: v["armor"].as_u64().unwrap_or(0) as i32,
            return_damage: v["return_damage"].as_u64().unwrap_or(0) as i32,
            str: v["str"].as_u64().unwrap_or(0) as i32,
            intl: v["intl"].as_u64().unwrap_or(0) as i32,
            wis: v["wis"].as_u64().unwrap_or(0) as i32,
            agi: v["agi"].as_u64().unwrap_or(0) as i32,
            con: v["con"].as_u64().unwrap_or(0) as i32,
            cha: v["cha"].as_u64().unwrap_or(0) as i32,
            light_resistance: v["light_resistance"].as_u64().unwrap_or(0) as i32,
            dark_resistance: v["dark_resistance"].as_u64().unwrap_or(0) as i32,
            earth_resistance: v["earth_resistance"].as_u64().unwrap_or(0) as i32,
            air_resistance: v["air_resistance"].as_u64().unwrap_or(0) as i32,
            water_resistance: v["water_resistance"].as_u64().unwrap_or(0) as i32,
            fire_resistance: v["fire_resistance"].as_u64().unwrap_or(0) as i32,
            spec1: v["spec1"].as_u64().unwrap_or(0) as i32,
            spec2: v["spec2"].as_u64().unwrap_or(0) as i32,
            spec3: v["spec3"].as_u64().unwrap_or(0) as i32,
            level_requirement: v["level_requirement"].as_u64().unwrap_or(0) as i32,
            class_requirement: v["class_requirement"].as_u64().unwrap_or(0) as i32,
            str_requirement: v["str_requirement"].as_u64().unwrap_or(0) as i32,
            int_requirement: v["int_requirement"].as_u64().unwrap_or(0) as i32,
            wis_requirement: v["wis_requirement"].as_u64().unwrap_or(0) as i32,
            agi_requirement: v["agi_requirement"].as_u64().unwrap_or(0) as i32,
            con_requirement: v["con_requirement"].as_u64().unwrap_or(0) as i32,
            cha_requirement: v["cha_requirement"].as_u64().unwrap_or(0) as i32,
            element: Element::from(v["element"].as_u64().unwrap_or(0) as i32),
            element_damage: v["element_damage"].as_u64().unwrap_or(0) as i32,
            weight: v["weight"].as_u64().unwrap_or(0) as i32,
            size: ItemSize::from(v["size"].as_u64().unwrap_or(0) as i32),
        });
    }
    eif.items.push(EifRecord {
        name: "eof".to_string(),
        ..Default::default()
    });
    eif.total_items_count = eif.items.len() as i32;
    let encoded = set_crc32_rid(&mut eif);
    eif.rid = [
        decode_number(&encoded[0..=1]) as i32,
        decode_number(&encoded[2..=3]) as i32,
    ];
    save_pub_file(&eif, &format!("{}/dat001.eif", pub_path))?;
    println!("✨ Generated dat001.eif ({} items)", entries.len());
    Ok(())
}

fn json2pub_npcs(json_path: &str, pub_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut enf = Enf::default();
    let mut drop_file = DropFile::default();
    let mut talk_file = TalkFile::default();

    let pattern = Path::new(json_path).join("npcs/*.json");
    let mut entries: Vec<_> = glob(pattern.to_str().unwrap())?
        .filter_map(|e| e.ok())
        .collect();
    entries.sort();

    let mut npc_id = 1;
    for entry in &entries {
        let v: Value = serde_json::from_str(&std::fs::read_to_string(entry)?)?;
        enf.npcs.push(EnfRecord {
            name: v["name"].as_str().unwrap_or_default().to_string(),
            graphic_id: v["graphic_id"].as_u64().unwrap_or(0) as i32,
            race: v["race"].as_u64().unwrap_or(0) as i32,
            boss: v["boss"].as_bool().unwrap_or(false),
            child: v["child"].as_bool().unwrap_or(false),
            r#type: NpcType::from(v["type"].as_u64().unwrap_or(0) as i32),
            behavior_id: v["behavior_id"].as_u64().unwrap_or(0) as i32,
            hp: v["hp"].as_u64().unwrap_or(0) as i32,
            tp: v["tp"].as_u64().unwrap_or(0) as i32,
            min_damage: v["min_damage"].as_u64().unwrap_or(0) as i32,
            max_damage: v["max_damage"].as_u64().unwrap_or(0) as i32,
            accuracy: v["accuracy"].as_u64().unwrap_or(0) as i32,
            evade: v["evade"].as_u64().unwrap_or(0) as i32,
            armor: v["armor"].as_u64().unwrap_or(0) as i32,
            return_damage: v["return_damage"].as_u64().unwrap_or(0) as i32,
            element: Element::from(v["element"].as_u64().unwrap_or(0) as i32),
            element_damage: v["element_damage"].as_u64().unwrap_or(0) as i32,
            element_weakness: Element::from(v["element_weakness"].as_u64().unwrap_or(0) as i32),
            element_weakness_damage: v["element_weakness_damage"].as_u64().unwrap_or(0) as i32,
            level: v["level"].as_u64().unwrap_or(0) as i32,
            experience: v["experience"].as_u64().unwrap_or(0) as i32,
        });

        if let Some(drops) = v["drops"].as_array() {
            if !drops.is_empty() {
                drop_file.npcs.push(DropNpcRecord {
                    npc_id,
                    drops: drops
                        .iter()
                        .map(|d| DropRecord {
                            item_id: d["item_id"].as_u64().unwrap_or(0) as i32,
                            min_amount: d["min_amount"].as_u64().unwrap_or(0) as i32,
                            max_amount: d["max_amount"].as_u64().unwrap_or(0) as i32,
                            rate: d["rate"].as_u64().unwrap_or(0) as i32,
                        })
                        .collect(),
                });
            }
        }

        if let Some(messages) = v["talk_messages"].as_array() {
            if !messages.is_empty() {
                talk_file.npcs.push(TalkRecord {
                    npc_id,
                    rate: v["talk_rate"].as_u64().unwrap_or(0) as i32,
                    messages: messages
                        .iter()
                        .map(|m| TalkMessageRecord {
                            message: m.as_str().unwrap_or_default().to_string(),
                        })
                        .collect(),
                });
            }
        }

        npc_id += 1;
    }

    enf.npcs.push(EnfRecord {
        name: "eof".to_string(),
        ..Default::default()
    });
    enf.total_npcs_count = enf.npcs.len() as i32;
    let encoded = set_crc32_rid(&mut enf);
    enf.rid = [
        decode_number(&encoded[0..=1]) as i32,
        decode_number(&encoded[2..=3]) as i32,
    ];
    save_pub_file(&enf, &format!("{}/dtn001.enf", pub_path))?;
    save_pub_file(&drop_file, &format!("{}/dtd001.edf", pub_path))?;
    save_pub_file(&talk_file, &format!("{}/ttd001.etf", pub_path))?;
    println!(
        "✨ Generated dtn001.enf, dtd001.edf, ttd001.etf ({} npcs)",
        entries.len()
    );
    Ok(())
}

fn json2pub_shops(json_path: &str, pub_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut shop_file = ShopFile::default();
    let pattern = Path::new(json_path).join("shops/*.json");
    let mut entries: Vec<_> = glob(pattern.to_str().unwrap())?
        .filter_map(|e| e.ok())
        .collect();
    entries.sort();
    for entry in &entries {
        let v: Value = serde_json::from_str(&std::fs::read_to_string(entry)?)?;
        let trades = v["trades"].as_array().cloned().unwrap_or_default();
        let crafts = v["crafts"].as_array().cloned().unwrap_or_default();
        shop_file.shops.push(ShopRecord {
            behavior_id: v["behavior_id"].as_u64().unwrap_or(0) as i32,
            name: v["name"].as_str().unwrap_or_default().to_string(),
            min_level: v["min_level"].as_u64().unwrap_or(0) as i32,
            max_level: v["max_level"].as_u64().unwrap_or(0) as i32,
            class_requirement: v["class_requirement"].as_u64().unwrap_or(0) as i32,
            trades: trades
                .iter()
                .map(|t| ShopTradeRecord {
                    item_id: t["item_id"].as_u64().unwrap_or(0) as i32,
                    buy_price: t["buy_price"].as_u64().unwrap_or(0) as i32,
                    sell_price: t["sell_price"].as_u64().unwrap_or(0) as i32,
                    max_amount: t["max_amount"].as_u64().unwrap_or(0) as i32,
                })
                .collect(),
            crafts: crafts
                .iter()
                .map(|c| {
                    let ingredients = c["ingredients"].as_array().cloned().unwrap_or_default();
                    let mut arr = [const {
                        ShopCraftIngredientRecord {
                            item_id: 0,
                            amount: 0,
                        }
                    }; 4];
                    for (i, ing) in ingredients.iter().enumerate().take(4) {
                        arr[i] = ShopCraftIngredientRecord {
                            item_id: ing["item_id"].as_u64().unwrap_or(0) as i32,
                            amount: ing["amount"].as_u64().unwrap_or(0) as i32,
                        };
                    }
                    ShopCraftRecord {
                        item_id: c["item_id"].as_u64().unwrap_or(0) as i32,
                        ingredients: arr,
                    }
                })
                .collect(),
        });
    }
    save_pub_file(&shop_file, &format!("{}/dts001.esf", pub_path))?;
    println!("✨ Generated dts001.esf ({} shops)", entries.len());
    Ok(())
}

fn json2pub_inns(json_path: &str, pub_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut inn_file = InnFile::default();
    let pattern = Path::new(json_path).join("inns/*.json");
    let mut entries: Vec<_> = glob(pattern.to_str().unwrap())?
        .filter_map(|e| e.ok())
        .collect();
    entries.sort();
    for entry in &entries {
        let v: Value = serde_json::from_str(&std::fs::read_to_string(entry)?)?;
        let questions = v["questions"].as_array().cloned().unwrap_or_default();
        let mut q_arr = [const {
            InnQuestionRecord {
                question: String::new(),
                answer: String::new(),
            }
        }; 3];
        for (i, q) in questions.iter().enumerate().take(3) {
            q_arr[i] = InnQuestionRecord {
                question: q["question"].as_str().unwrap_or_default().to_string(),
                answer: q["answer"].as_str().unwrap_or_default().to_string(),
            };
        }
        inn_file.inns.push(InnRecord {
            behavior_id: v["behavior_id"].as_u64().unwrap_or(0) as i32,
            name: v["name"].as_str().unwrap_or_default().to_string(),
            spawn_map: v["spawn_map"].as_u64().unwrap_or(0) as i32,
            spawn_x: v["spawn_x"].as_u64().unwrap_or(0) as i32,
            spawn_y: v["spawn_y"].as_u64().unwrap_or(0) as i32,
            sleep_map: v["sleep_map"].as_u64().unwrap_or(0) as i32,
            sleep_x: v["sleep_x"].as_u64().unwrap_or(0) as i32,
            sleep_y: v["sleep_y"].as_u64().unwrap_or(0) as i32,
            alternate_spawn_enabled: v["alternate_spawn_enabled"].as_bool().unwrap_or(false),
            alternate_spawn_map: v["alternate_spawn_map"].as_u64().unwrap_or(0) as i32,
            alternate_spawn_x: v["alternate_spawn_x"].as_u64().unwrap_or(0) as i32,
            alternate_spawn_y: v["alternate_spawn_y"].as_u64().unwrap_or(0) as i32,
            questions: q_arr,
        });
    }
    save_pub_file(&inn_file, &format!("{}/din001.eid", pub_path))?;
    println!("✨ Generated din001.eid ({} inns)", entries.len());
    Ok(())
}

fn json2pub_skill_masters(
    json_path: &str,
    pub_path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut sm_file = SkillMasterFile::default();
    let pattern = Path::new(json_path).join("skill_masters/*.json");
    let mut entries: Vec<_> = glob(pattern.to_str().unwrap())?
        .filter_map(|e| e.ok())
        .collect();
    entries.sort();
    for entry in &entries {
        let v: Value = serde_json::from_str(&std::fs::read_to_string(entry)?)?;
        let skills = v["skills"].as_array().cloned().unwrap_or_default();
        sm_file.skill_masters.push(SkillMasterRecord {
            behavior_id: v["behavior_id"].as_u64().unwrap_or(0) as i32,
            name: v["name"].as_str().unwrap_or_default().to_string(),
            min_level: v["min_level"].as_u64().unwrap_or(0) as i32,
            max_level: v["max_level"].as_u64().unwrap_or(0) as i32,
            class_requirement: v["class_requirement"].as_u64().unwrap_or(0) as i32,
            skills: skills
                .iter()
                .map(|s| {
                    let reqs = s["skill_requirements"]
                        .as_array()
                        .cloned()
                        .unwrap_or_default();
                    let mut req_arr = [0i32; 4];
                    for (i, r) in reqs.iter().enumerate().take(4) {
                        req_arr[i] = r.as_u64().unwrap_or(0) as i32;
                    }
                    SkillMasterSkillRecord {
                        skill_id: s["id"].as_u64().unwrap_or(0) as i32,
                        level_requirement: s["level_requirement"].as_u64().unwrap_or(0) as i32,
                        class_requirement: s["class_requirement"].as_u64().unwrap_or(0) as i32,
                        price: s["price"].as_u64().unwrap_or(0) as i32,
                        skill_requirements: req_arr,
                        str_requirement: s["str_requirement"].as_u64().unwrap_or(0) as i32,
                        int_requirement: s["int_requirement"].as_u64().unwrap_or(0) as i32,
                        wis_requirement: s["wis_requirement"].as_u64().unwrap_or(0) as i32,
                        agi_requirement: s["agi_requirement"].as_u64().unwrap_or(0) as i32,
                        con_requirement: s["con_requirement"].as_u64().unwrap_or(0) as i32,
                        cha_requirement: s["cha_requirement"].as_u64().unwrap_or(0) as i32,
                    }
                })
                .collect(),
        });
    }
    save_pub_file(&sm_file, &format!("{}/dsm001.emf", pub_path))?;
    println!("✨ Generated dsm001.emf ({} skill masters)", entries.len());
    Ok(())
}

// ─── load binary pub files ────────────────────────────────────────────────────

fn load_class_file(path: &str) -> Result<Ecf, Box<dyn std::error::Error>> {
    let mut buf = Vec::new();
    File::open(Path::new(path).join("dat001.ecf"))?.read_to_end(&mut buf)?;
    Ok(Ecf::deserialize(&EoReader::new(Bytes::from(buf)))?)
}

fn load_drop_file(path: &str) -> Result<DropFile, Box<dyn std::error::Error>> {
    let mut buf = Vec::new();
    File::open(Path::new(path).join("dtd001.edf"))?.read_to_end(&mut buf)?;
    Ok(DropFile::deserialize(&EoReader::new(Bytes::from(buf)))?)
}

fn load_inn_file(path: &str) -> Result<InnFile, Box<dyn std::error::Error>> {
    let mut buf = Vec::new();
    File::open(Path::new(path).join("din001.eid"))?.read_to_end(&mut buf)?;
    Ok(InnFile::deserialize(&EoReader::new(Bytes::from(buf)))?)
}

fn load_item_file(path: &str) -> Result<Eif, Box<dyn std::error::Error>> {
    let mut buf = Vec::new();
    File::open(Path::new(path).join("dat001.eif"))?.read_to_end(&mut buf)?;
    Ok(Eif::deserialize(&EoReader::new(Bytes::from(buf)))?)
}

fn load_npc_file(path: &str) -> Result<Enf, Box<dyn std::error::Error>> {
    let mut buf = Vec::new();
    File::open(Path::new(path).join("dtn001.enf"))?.read_to_end(&mut buf)?;
    Ok(Enf::deserialize(&EoReader::new(Bytes::from(buf)))?)
}

fn load_shop_file(path: &str) -> Result<ShopFile, Box<dyn std::error::Error>> {
    let mut buf = Vec::new();
    File::open(Path::new(path).join("dts001.esf"))?.read_to_end(&mut buf)?;
    Ok(ShopFile::deserialize(&EoReader::new(Bytes::from(buf)))?)
}

fn load_skill_master_file(path: &str) -> Result<SkillMasterFile, Box<dyn std::error::Error>> {
    let mut buf = Vec::new();
    File::open(Path::new(path).join("dsm001.emf"))?.read_to_end(&mut buf)?;
    Ok(SkillMasterFile::deserialize(&EoReader::new(Bytes::from(
        buf,
    )))?)
}

fn load_spell_file(path: &str) -> Result<Esf, Box<dyn std::error::Error>> {
    let mut buf = Vec::new();
    File::open(Path::new(path).join("dsl001.esf"))?.read_to_end(&mut buf)?;
    Ok(Esf::deserialize(&EoReader::new(Bytes::from(buf)))?)
}

fn load_talk_file(path: &str) -> Result<TalkFile, Box<dyn std::error::Error>> {
    let mut buf = Vec::new();
    File::open(Path::new(path).join("ttd001.etf"))?.read_to_end(&mut buf)?;
    Ok(TalkFile::deserialize(&EoReader::new(Bytes::from(buf)))?)
}
