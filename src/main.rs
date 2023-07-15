use std::process::Command;
use std::fs;
use std::time;
use std::vec;
use std::collections::HashMap;
use id3::Timestamp;
use id3::{Tag, TagLike, Version};
use clap::Parser;
use eject::device::{Device, DriveStatus};
use read_input::prelude::*;
extern crate eject;

#[derive(Parser)]
#[command(name = "cda_copy")]
#[command(author="Mia")]
struct Cli{
    #[arg(short, long)]
    output: String,

    #[arg(short,long,default_value_t=str::to_string("sr0"))]
    drive: String,

    #[arg(short,long,default_value_t=str::to_string("192k"))]
    bitrate: String
}

fn main(){

    let cli = Cli::parse();

    let mut software = CDACopy::new(cli);

    software.prepare_disk_drive();
    software.get_track_list();
    software.create_temp_folder();
    software.aquire_tags();
    software.copy_to_temp_folder();
    software.combine_files();
    software.convert2mp3();
    software.write_id3_tags();
    software.remove_tmp_folder();
}


fn sys_time_in_secs()->u64{
    match time::SystemTime::now().duration_since(time::SystemTime::UNIX_EPOCH) {
        Ok(n) => n.as_secs(),
        Err(_) => panic!("SystemTime before UNIX EPOCH!"),
    }
}

pub struct CDACopy{
    pub output: String,
    pub drive: String,
    pub bitrate: String,
    pub drive_dev: Device,
    drive_str: String,
    temp_folder_name: String,
    tracklist: Vec<String>,
    id3_tags: HashMap<String,String>
}
impl CDACopy{
    fn new(cli:Cli)->CDACopy{
        let drive_dev:Device;
        match Device::open(format!("/dev/{}", cli.drive)) {
            Ok(result) => drive_dev = result,
            Err(err) => {
                panic!("Drive not found: {}", err)
            }
        }

        CDACopy{drive:String::from(&cli.drive),
                output:String::from(&cli.output), 
                bitrate: String::from(&cli.bitrate),
                temp_folder_name:"".to_string(), 
                tracklist:vec![], 
                drive_str:"".to_string(),
                drive_dev:drive_dev,
                id3_tags: HashMap::new()
            }
    }


    fn prepare_disk_drive(&mut self){
        if self.drive_dev.status().unwrap() == DriveStatus::TrayOpen{
            self.toggle_eject_disc();
        }
        self.drive_str = format!("cdda://{}/",self.drive).to_string();

        match Command::new("gio").args(["mount",&self.drive_str]).output(){
            Ok(_) => println!("Sucessfully mounted drive"),
            Err(err) => panic!("Error mounting drive: {}", err)
        };
    }


    fn get_track_list(&mut self){
        let tracklist = Command::new("gio").args(["list",&self.drive_str]).output().expect("Failed to read tracks on disk.\n").stdout;
        match String::from_utf8(tracklist){
            Ok(result) => self.tracklist = result.lines().map(str::to_string).collect(),
            Err(err) => panic!("Failed to obtain tracklist: {}", err)
        }
        println!("Found {} Tracks", &self.tracklist.len());
    }


    fn create_temp_folder(&mut self){
        self.temp_folder_name = format!("{}{}",".tmp",sys_time_in_secs());
        fs::create_dir(&self.temp_folder_name).expect("Couldn't create a temp folder\n");
    }


    fn copy_to_temp_folder(&self){
        let mut track_counter = 0;
        for track in &self.tracklist{
            let target_loc = format!("{}{}{}",self.temp_folder_name,"/",&track);
            let data = fs::read(format!("/run/user/1000/gvfs/cdda:host={}/{}",&self.drive,&track)).expect("Reading track failed\n");
            fs::write(&target_loc, data).expect("Writing track failed\n");
            track_counter+=1;
            println!("Copied {:?} ({}/{})", &track, &track_counter, &self.tracklist.len());
        }
        self.toggle_eject_disc()
    }


    fn toggle_eject_disc(&self){
        self.drive_dev.toggle_eject().unwrap();
    }


    fn combine_files(&self){
        let mut command_opts:Vec<String> = vec![];
        for track in &self.tracklist{
            command_opts.push(format!("{}/{}",self.temp_folder_name,track.to_string().replace(" ", "\\ ")));
        }
        command_opts.push(format!("{}/{}",self.temp_folder_name, "tmp.wav"));


        match Command::new("sox").args(command_opts).spawn(){
            Ok(_) => println!("Successfully combined files"),
            Err(error) => panic!("Error combining files: {}", error)
        };
    }

    fn remove_tmp_folder(&self){
        match fs::remove_dir_all(&self.temp_folder_name){
            Ok(_result) => println!("Removed temporary files"),
            Err(error) => println!("Error removing temporary files: {}",error)
        };
    }

    fn convert2mp3(&self){
        Command::new("ffmpeg").args(["-y","-i",&format!("{}/{}",self.temp_folder_name, "tmp.wav"), "-b:a", &self.bitrate, "-c:a","mp3",&self.output]).output().expect("Failed to convert audio file format\n");
    }

    fn aquire_tags(&mut self){
        println!("Insert ID3 Tags:");
        print!("Title: ");
        self.id3_tags.insert(
            "title".to_string(),
            input().get()
        );
        print!("Album: ");
        self.id3_tags.insert(
            "album".to_string(),
            input().get()
        );
        print!("Album Artist: ");
        self.id3_tags.insert(
            "artist".to_string(),
            input().get()
        );
        print!("Year: ");
        let year_tag = input::<i32>().get();
        self.id3_tags.insert(
            "year".to_string(),
            year_tag.to_string()
        );

        //println!("{:?}",self.id3_tags);
    }


    fn write_id3_tags(&self){
        let mut tag = Tag::new();
        tag.set_album(self.id3_tags.get("album").unwrap());
        tag.set_title(self.id3_tags.get("title").unwrap());
        tag.set_artist(self.id3_tags.get("artist").unwrap());
        let year = Timestamp{year: self.id3_tags.get("year").unwrap().parse().unwrap(), month: None, day: None, hour: None, minute: None, second: None};
        tag.set_date_released(year);
        tag.write_to_path(&self.output, Version::Id3v24).expect("Failed to write ID3-Tags");
    }
}