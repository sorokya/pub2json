use bytes::Bytes;
use clap::Parser;
use serde_json::json;
use std::{fs::File, io::Read, path::Path};

use eo::{
    data::{Serializeable, StreamReader},
    pubs::{
        DropFile, DropNpc, EcfFile, EifFile, EnfFile, EsfFile, InnFile, ShopFile, SkillMasterFile,
        TalkFile, TalkNpc,
    },
};

/// A little tool to convert EO data files to JSON
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// path to directory containing pub files
    #[arg(short, long, default_value = "./pub")]
    pubs: String,

    /// path to directory to dump the JSON files
    #[arg(short, long, default_value = "./pub_json")]
    output: String,

    /// What type of server data files are you converting
    #[arg(short, long, default_value = "original")]
    server: ServerData,
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

    let class_file = load_class_file(&args.pubs);
    let drop_file = match load_drop_file(&args.pubs) {
        Ok(drop_file) => drop_file,
        Err(_) => {
            println!("Could not load drop file, using default");
            DropFile::default()
        }
    };
    let inn_file = match load_inn_file(&args.pubs) {
        Ok(inn_file) => inn_file,
        Err(_) => {
            println!("Could not load inn file, using default");
            InnFile::default()
        }
    };
    let item_file = load_item_file(&args.pubs);
    let npc_file = load_npc_file(&args.pubs);
    let shop_file = match load_shop_file(&args.pubs) {
        Ok(shop_file) => shop_file,
        Err(_) => {
            println!("Could not load shop file, using default");
            ShopFile::default()
        }
    };
    let skill_master_file = match load_skill_master_file(&args.pubs) {
        Ok(skill_master_file) => skill_master_file,
        Err(_) => {
            println!("Could not load skill master file, using default");
            SkillMasterFile::default()
        }
    };
    let spell_file = load_spell_file(&args.pubs);
    let talk_file = match load_talk_file(&args.pubs) {
        Ok(talk_file) => talk_file,
        Err(_) => {
            println!("Could not load talk file, using default");
            TalkFile::default()
        }
    };

    match class_file {
        Ok(class_file) => {
            let _ = generate_class_json(&class_file, &args.output);
        }
        Err(e) => println!("Could not load class file: {}", e),
    }

    match spell_file {
        Ok(spell_file) => {
            let _ = generate_spell_json(&spell_file, &args.output);
        }
        Err(e) => println!("Could not load spell file: {}", e),
    }

    match item_file {
        Ok(item_file) => {
            let _ = generate_item_json(&item_file, &args.output);
        }
        Err(e) => println!("Could not load item file: {}", e),
    }

    match npc_file {
        Ok(npc_file) => {
            let _ = generate_npc_json(&npc_file, &drop_file, &talk_file, &args.output);
        }
        Err(e) => println!("Could not load npc file: {}", e),
    }

    let _ = generate_shop_json(&shop_file, &args.output);
    let _ = generate_inn_json(&inn_file, &args.output);
    let _ = generate_skill_master_json(&skill_master_file, &args.output);
}

fn generate_class_json(class_file: &EcfFile, path: &str) -> Result<(), Box<dyn std::error::Error>> {
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
            "parent": class.parent_type,
            "type": class.r#type.to_char(),
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

fn generate_spell_json(spell_file: &EsfFile, path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let dir = Path::new(path).join("spells");
    if dir.exists() {
        std::fs::remove_dir_all(dir.clone())?;
    }

    std::fs::create_dir_all(dir.clone())?;

    let mut id = 1;
    for spell in &spell_file.spells {
        if spell.name == "eof" {
            continue;
        }

        let json = json!({
            "name": spell.name,
            "shout": spell.shout,
            "iconId": spell.icon_id,
            "graphicId": spell.graphic_id,
            "tpCost": spell.tp_cost,
            "spCost": spell.sp_cost,
            "castTime": spell.cast_time,
            "nature": spell.nature.to_char(),
            "type": spell.r#type.to_three(),
            "element": spell.element,
            "elementPower": spell.element_power,
            "targetRestrict": spell.target_restrict.to_char(),
            "targetType": spell.target_type.to_char(),
            "targetTime": spell.target_time,
            "maxSkillLevel": spell.max_skill_level,
            "minDamage": spell.min_damage,
            "maxDamage": spell.max_damage,
            "accuracy": spell.accuracy,
            "evade": spell.evade,
            "armor": spell.armor,
            "returnDamage": spell.return_damage,
            "healHp": spell.hp_heal,
            "healTp": spell.tp_heal,
            "healSp": spell.sp_heal,
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

fn generate_item_json(item_file: &EifFile, path: &str) -> Result<(), Box<dyn std::error::Error>> {
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
            "graphicId": item.graphic_id,
            "type": item.r#type.to_char(),
            "subType": item.subtype.to_char(),
            "special": item.special.to_char(),
            "hp": item.hp,
            "tp": item.tp,
            "minDamage": item.min_damage,
            "maxDamage": item.max_damage,
            "accuracy": item.accuracy,
            "evade": item.evade,
            "armor": item.armor,
            "returnDamage": item.return_damage,
            "str": item.str,
            "intl": item.intl,
            "wis": item.wis,
            "agi": item.agi,
            "con": item.con,
            "cha": item.cha,
            "lightResistance": item.light_resistance,
            "darkResistance": item.dark_resistance,
            "earthResistance": item.earth_resistance,
            "airResistance": item.air_resistance,
            "waterResistance": item.water_resistance,
            "fireResistance": item.fire_resistance,
            "spec1": item.spec1,
            "spec2": item.spec2,
            "spec3": item.spec3,
            "strReq": item.str_req,
            "intReq": item.int_req,
            "wisReq": item.wis_req,
            "agiReq": item.agi_req,
            "conReq": item.con_req,
            "chaReq": item.cha_req,
            "element": item.element,
            "elementDamage": item.element_damage,
            "weight": item.weight,
            "size": item.size.to_char(),
        });

        std::fs::write(
            dir.join(format!("{:0>4}.json", id - 1)),
            serde_json::to_string_pretty(&json).unwrap(),
        )?;

        id += 1;
    }

    println!("✨ Generated {} item files", id - 1);

    Ok(())
}

fn generate_npc_json(
    npc_file: &EnfFile,
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

        let drop_record = match drop_file.npcs.iter().find(|n| n.npc_id == id) {
            Some(n) => n.to_owned(),
            None => DropNpc::default(),
        };

        let talk_record = match talk_file.npcs.iter().find(|n| n.npc_id == id) {
            Some(n) => n.to_owned(),
            None => TalkNpc::default(),
        };

        let json = json!({
            "name": npc.name,
            "graphicId": npc.graphic_id,
            "race": npc.race,
            "boss": npc.boss,
            "child": npc.child,
            "type": npc.r#type.to_short(),
            "behaviorId": npc.behavior_id,
            "hp": npc.hp,
            "tp": npc.tp,
            "minDamage": npc.min_damage,
            "maxDamage": npc.max_damage,
            "accuracy": npc.accuracy,
            "evade": npc.evade,
            "armor": npc.armor,
            "returnDamage": npc.return_damage,
            "element": npc.element,
            "elementDamage": npc.element_damage,
            "elementWeakness": npc.element_weakness,
            "elementWeaknessDamage": npc.element_weakness_damage,
            "drops": drop_record.drops.iter().map(|drop| {
                json!({
                    "itemId": drop.item_id,
                    "min": drop.min,
                    "max": drop.max,
                    "rate": drop.rate,
                })
            }).collect::<Vec<_>>(),
            "talkRate": talk_record.rate,
            "talkMessages": talk_record.messages,
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
            "behaviorId": shop.vendor_id,
            "name": shop.name,
            "minLevel": shop.min_level,
            "maxLevel": shop.max_level,
            "classReq": shop.class_req,
            "trades": shop.trades.iter().map(|trade| {
                json!({
                    "itemId": trade.item_id,
                    "buyPrice": trade.buy_price,
                    "sellPrice": trade.sell_price,
                    "maxAmount": trade.max_amount,
                })
            }).collect::<Vec<_>>(),
            "crafts": shop.crafts.iter().map(|craft| {
                json!({
                    "itemId": craft.item_id,
                    "ingredient1ItemId": craft.ingredient1_item_id,
                    "ingredient1Amount": craft.ingredient1_amount,
                    "ingredient2ItemId": craft.ingredient2_item_id,
                    "ingredient2Amount": craft.ingredient2_amount,
                    "ingredient3ItemId": craft.ingredient3_item_id,
                    "ingredient3Amount": craft.ingredient3_amount,
                    "ingredient4ItemId": craft.ingredient4_item_id,
                    "ingredient4Amount": craft.ingredient4_amount,
                })
            }).collect::<Vec<_>>(),
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
            "behaviorId": inn.vendor_id,
            "name": inn.name,
            "spawnMap": inn.spawn_map,
            "spawnX": inn.spawn_x,
            "spawnY": inn.spawn_y,
            "sleepMap": inn.sleep_map,
            "sleepX": inn.sleep_x,
            "sleepY": inn.sleep_y,
            "altSpawnEnabled": inn.alt_spawn_enabled,
            "altSpawnMap": inn.alt_spawn_map,
            "altSpawnX": inn.alt_spawn_x,
            "altSpawnY": inn.alt_spawn_y,
            "question1": inn.question1,
            "answer1": inn.answer1,
            "question2": inn.question2,
            "answer2": inn.answer2,
            "question3": inn.question3,
            "answer3": inn.answer3,
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
    for skill_master in &skill_master_file.skill_masters {
        if skill_master.name == "eof" {
            continue;
        }

        let json = json!({
            "behaviorId": skill_master.vendor_id,
            "name": skill_master.name,
            "minLevel": skill_master.min_level,
            "maxLevel": skill_master.max_level,
            "classReq": skill_master.class_req,
            "skills": skill_master.skills.iter().map(|s| json!({
                "id": s.skill_id,
                "minLevel": s.min_level,
                "classReq": s.class_req,
                "price": s.price,
                "skillIdReq1": s.skill_id_req1,
                "skillIdReq2": s.skill_id_req2,
                "skillIdReq3": s.skill_id_req3,
                "skillIdReq4": s.skill_id_req4,
                "strReq": s.str_req,
                "intReq": s.int_req,
                "wisReq": s.wis_req,
                "agiReq": s.agi_req,
                "conReq": s.con_req,
                "chaReq": s.cha_req,
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

fn load_class_file(path: &str) -> Result<EcfFile, Box<dyn std::error::Error>> {
    let path = Path::new(path);
    let path = path.join("dat001.ecf");
    let mut file = File::open(path)?;
    let mut buf = Vec::new();
    file.read_to_end(&mut buf)?;

    let bytes = Bytes::from(buf);

    let reader = StreamReader::new(bytes);

    let mut ecf_file = EcfFile::default();
    ecf_file.deserialize(&reader);
    Ok(ecf_file)
}

fn load_drop_file(path: &str) -> Result<DropFile, Box<dyn std::error::Error>> {
    let path = Path::new(path);
    let mut file = File::open(path.join("dtd001.edf"))?;
    let mut buf = Vec::new();
    file.read_to_end(&mut buf)?;

    let bytes = Bytes::from(buf);
    let reader = StreamReader::new(bytes);

    let mut drop_file = DropFile::default();
    drop_file.deserialize(&reader);
    Ok(drop_file)
}

fn load_inn_file(path: &str) -> Result<InnFile, Box<dyn std::error::Error>> {
    let path = Path::new(path);
    let mut file = File::open(path.join("din001.eid"))?;
    let mut buf = Vec::new();
    file.read_to_end(&mut buf)?;

    let bytes = Bytes::from(buf);
    let reader = StreamReader::new(bytes);

    let mut inn_file = InnFile::default();
    inn_file.deserialize(&reader);
    Ok(inn_file)
}

fn load_item_file(path: &str) -> Result<EifFile, Box<dyn std::error::Error>> {
    let path = Path::new(path);
    let mut file = File::open(path.join("dat001.eif"))?;
    let mut buf = Vec::new();
    file.read_to_end(&mut buf)?;

    let bytes = Bytes::from(buf);
    let reader = StreamReader::new(bytes);

    let mut item_file = EifFile::default();
    item_file.deserialize(&reader);
    Ok(item_file)
}

fn load_npc_file(path: &str) -> Result<EnfFile, Box<dyn std::error::Error>> {
    let path = Path::new(path);
    let mut file = File::open(path.join("dtn001.enf"))?;
    let mut buf = Vec::new();
    file.read_to_end(&mut buf)?;

    let bytes = Bytes::from(buf);
    let reader = StreamReader::new(bytes);

    let mut npc_file = EnfFile::default();
    npc_file.deserialize(&reader);
    Ok(npc_file)
}

fn load_shop_file(path: &str) -> Result<ShopFile, Box<dyn std::error::Error>> {
    let path = Path::new(path);
    let mut file = File::open(path.join("dts001.esf"))?;
    let mut buf = Vec::new();
    file.read_to_end(&mut buf)?;

    let bytes = Bytes::from(buf);
    let reader = StreamReader::new(bytes);

    let mut shop_file = ShopFile::default();
    shop_file.deserialize(&reader);
    Ok(shop_file)
}

fn load_skill_master_file(path: &str) -> Result<SkillMasterFile, Box<dyn std::error::Error>> {
    let path = Path::new(path);
    let mut file = File::open(path.join("dsm001.emf"))?;
    let mut buf = Vec::new();
    file.read_to_end(&mut buf)?;

    let bytes = Bytes::from(buf);
    let reader = StreamReader::new(bytes);

    let mut skill_master_file = SkillMasterFile::default();
    skill_master_file.deserialize(&reader);
    Ok(skill_master_file)
}

fn load_spell_file(path: &str) -> Result<EsfFile, Box<dyn std::error::Error>> {
    let path = Path::new(path);
    let mut file = File::open(path.join("dsl001.esf"))?;
    let mut buf = Vec::new();
    file.read_to_end(&mut buf)?;

    let bytes = Bytes::from(buf);
    let reader = StreamReader::new(bytes);

    let mut spell_file = EsfFile::default();
    spell_file.deserialize(&reader);
    Ok(spell_file)
}

fn load_talk_file(path: &str) -> Result<TalkFile, Box<dyn std::error::Error>> {
    let path = Path::new(path);
    let mut file = File::open(path.join("ttd001.etf"))?;
    let mut buf = Vec::new();
    file.read_to_end(&mut buf)?;

    let bytes = Bytes::from(buf);
    let reader = StreamReader::new(bytes);

    let mut talk_file = TalkFile::default();
    talk_file.deserialize(&reader);
    Ok(talk_file)
}
