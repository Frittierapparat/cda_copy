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

#[derive(Parser)]
#[command(name = "cda_copy")]
#[command(author="Mia")]
struct Cli{
    //Defines the output filename
    output: String,

    //Defines the drive to be used (should be improved to decide automatically)
    drive: String,

    #[arg(short,long,default_value_t=str::to_string("192k"))]
    //Bitrate for ffmpeg export
    bitrate: String
}

fn main(){
    let cli = Cli::parse();

    let mut cda_copy = CDACopy::new(cli.drive, cli.output, cli.bitrate);

    cda_copy.prepare_disk_drive();
    cda_copy.get_track_list();
    cda_copy.create_temp_folder();
    cda_copy.aquire_tags();
    cda_copy.copy_to_temp_folder();
    cda_copy.combine_files();
    cda_copy.convert2mp3();
    cda_copy.write_id3_tags();
    cda_copy.remove_tmp_folder();
}


fn sys_time_in_secs()->u64{
    //!Returns the System Time in seconds since the Unix Epoch (01/01/1970)

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
    fn new(drive: String, output: String, bitrate:String)->CDACopy{
        let drive_dev:Device;
        match Device::open(format!("/dev/{}", drive)) {
            Ok(result) => drive_dev = result,
            Err(err) => {
                panic!("Drive not found: {}", err)
            }
        }

        CDACopy{drive:String::from(&drive),
                output:String::from(&output), 
                bitrate: String::from(&bitrate),
                temp_folder_name:"".to_string(), 
                tracklist:vec![], 
                drive_str:"".to_string(),
                drive_dev:drive_dev,
                id3_tags: HashMap::new()
            }
    }


    fn prepare_disk_drive(&mut self){
        //!Checks if the disk drive is open, closes it if necessary,
        //! mounts the selected disk drive
        if self.drive_dev.status().unwrap() == DriveStatus::TrayOpen{
            self.toggle_eject_disc();
        };
        self.drive_str = format!("cdda://{}/",self.drive).to_string();

        match Command::new("gio").args(["mount",&self.drive_str]).output(){
            Ok(_) => println!("Sucessfully mounted drive"),
            Err(err) => panic!("Error mounting drive: {}", err)
        };
    }


    fn get_track_list(&mut self){
        //!Reads a list of all files using gio and converts it into a Vector
        let tracklist = Command::new("gio").args(["list",&self.drive_str]).output().expect("Failed to read tracks on disk.\n").stdout;
        match String::from_utf8(tracklist){
            Ok(result) => self.tracklist = result.lines().map(str::to_string).collect(),
            Err(err) => panic!("Failed to obtain tracklist: {}", err)
        }
        println!("Found {} Tracks", &self.tracklist.len());
    }


    fn create_temp_folder(&mut self){
        //!Creates a temporary folder (comprised of .tmp and the unix timestamp)
        self.temp_folder_name = format!("{}{}",".tmp",sys_time_in_secs());
        fs::create_dir(&self.temp_folder_name).expect("Couldn't create a temp folder\n");
    }


    fn copy_to_temp_folder(&self){
        //!Copies files from the tracklist into the temporary folder
        //! 
        //! Due to some issues encountered during writing, this happens by 
        //! reading an entire file into memory and writing it from memory.
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
        self.drive_dev.toggle_eject().expect("Failed to toggle the Disk tray!");
    }


    fn combine_files(&self){
        //!Combines the copied files using sox into `tmp.wav`
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
        //!Deletes the temporary folder
        match fs::remove_dir_all(&self.temp_folder_name){
            Ok(_result) => println!("Removed temporary files"),
            Err(error) => panic!("Error removing temporary files: {}",error)
        };
    }

    fn convert2mp3(&self){
        //!Converts the combined wav file into mp3 using ffmpeg
        Command::new("ffmpeg").args(["-y","-i",&format!("{}/{}",self.temp_folder_name, "tmp.wav"), "-b:a", &self.bitrate, "-c:a","mp3",&self.output]).output().expect("Failed to convert audio file format\n");
    }

    fn aquire_tags(&mut self){
        //!Asks the user to input the ID3 tags for writing into the file later
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
    }


    fn write_id3_tags(&self){
        //!Writes the ID3 tags into the final mp3 file
        let mut tag = Tag::new();
        tag.set_album(self.id3_tags.get("album").unwrap());
        tag.set_title(self.id3_tags.get("title").unwrap());
        tag.set_artist(self.id3_tags.get("artist").unwrap());
        let year = Timestamp{year: self.id3_tags.get("year").unwrap().parse().unwrap(), month: None, day: None, hour: None, minute: None, second: None};
        tag.set_date_released(year);
        tag.write_to_path(&self.output, Version::Id3v24).expect("Failed to write ID3-Tags");
    }
}