use std::io::{stdout, Write, Read, stdin};
use std::process::{Command, exit};
use std::fs;
use std::time;
use std::vec;
use std::collections::HashMap;
use id3::{Tag, TagLike, Version, Timestamp};
use clap::Parser;
use eject::device::{Device, DriveStatus};
use eject::discovery::cd_drives;
use read_input::prelude::*;
use wav_concat::wav_concat;

#[derive(Parser)]
#[command(name = "cda_copy")]
#[command(author="Mia")]
struct Cli{
    //Defines the output filename
    output: String,

    //Defines the drive to be used (should be improved to decide automatically)
    #[arg(short,long, default_value_t=str::to_string("sr0"))]
    drive: String,

    #[arg(short,long,default_value_t=str::to_string("192k"))]
    //Bitrate for ffmpeg export
    bitrate: String,

    #[arg(short,long)]
    skip_tagging: bool,

    #[arg(short,long, default_value_t=1)]
    num_disks: u16
}

fn main(){
    let cli = Cli::parse();

    let mut cda_copy = CDACopy::new(cli.drive, cli.output, cli.bitrate, &cli.num_disks);
    let temp_file_list = &cda_copy.temp_files.to_vec();
    //cda_copy.prepare_disk_drive();
    //cda_copy.get_track_list();
    //if cli.skip_tagging {} else{cda_copy.aquire_tags()};
    //cda_copy.create_temp_folder();
    //cda_copy.copy_to_temp_folder();
    //cda_copy.combine_files();
    //cda_copy.convert2mp3();
    //if cli.skip_tagging{} else{cda_copy.write_id3_tags()};
    //cda_copy.remove_tmp_folder();
    if cli.skip_tagging {} else{cda_copy.aquire_tags()};
    cda_copy.create_temp_folder();
    for disk in 0..cli.num_disks{
        //println!("{}", disk);
        cda_copy.prepare_disk_drive();
        cda_copy.get_track_list();
        cda_copy.copy_to_temp_folder();
        //println!("{:?}", &cda_copy.tracklist);

        let mut copied_filelist: Vec<String> = vec![];
        for track in &cda_copy.tracklist{
            copied_filelist.push(format!("{}/{}", &cda_copy.temp_folder_name, track));
        }
        cda_copy.combine_files(String::from(&cda_copy.temp_files[disk as usize]), &copied_filelist);
        cda_copy.clean_tmp_folder();
        if disk < cli.num_disks -1 {pause()};
    }
    //println!("{:?}", temp_file_list);
    let mut final_file_list: Vec<String> = vec![];
    for file in temp_file_list{
        final_file_list.push(format!("{}/{}", &cda_copy.temp_folder_name, file))
    }
    cda_copy.combine_files("tmp.wav".to_string(), &final_file_list);
    cda_copy.convert2mp3();
    if cli.skip_tagging{} else{cda_copy.write_id3_tags()};
    cda_copy.remove_tmp_folder()
}


fn pause()
{
    let mut stdout = stdout();
    stdout.write(b"Press any Key once you have inserted the next disk").unwrap();
    stdout.flush().unwrap();
    stdin().read(&mut [0]).unwrap();
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
    temp_files: Vec<String>,
    tracklist: Vec<String>,
    id3_tags: HashMap<String,String>
}
impl CDACopy{
    fn new(drive: String, output: String, bitrate:String, disk_num: &u16)->CDACopy{
        let drive_dev:Device;
        let drive_str: String;
        let drive_name: String;

        match Device::open(format!("/dev/{}", drive)){
            Ok(dev) => {
                drive_dev = dev;
                drive_name = drive;
                drive_str = format!("cdda://{}/",&drive_name);
            }
            Err(_) =>{
                println!("Drive not found, trying other drives…");
                let selected_drive = cd_drives().unwrap().next().unwrap();
                drive_dev = Device::open(&selected_drive).unwrap();
                let idx = selected_drive.to_string_lossy().find("sr").unwrap();
                drive_name = String::from_utf8(selected_drive.to_string_lossy().as_bytes()[idx..].to_vec()).unwrap();
                drive_str = format!("cdda://{}/",&drive_name);
            } 
        }


        let mut temp_file_vec: Vec<String> = vec![];
        for i in 0..*disk_num{
            temp_file_vec.push(format!("tmp{:03}.wav", i))
        }
        //println!("{:?}", temp_file_vec);
        CDACopy{
            drive:drive_name,
            output:String::from(&output), 
            bitrate: String::from(&bitrate),
            temp_folder_name:"".to_string(), 
            temp_files:temp_file_vec,
            tracklist:vec![], 
            drive_str:drive_str,
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

        match Command::new("gio").args(["mount",&self.drive_str]).output(){
            Ok(_) => println!("Sucessfully mounted drive"),
            Err(err) => panic!("Error mounting drive: {}", err)
        };
    }


    fn get_track_list(&mut self){
        //!Reads a list of all files using gio and converts it into a Vector
        let tracklist = Command::new("gio").args(["list",&self.drive_str]).output().expect("Failed to read tracks on disk.\n").stdout;
        match String::from_utf8(tracklist){
            Ok(result) => {
                if result.len() > 0{
                    self.tracklist = result.lines().map(str::to_string).collect()
                }
                else{
                    println!("No Tracks were found. Exiting...");
                    exit(1)
                }
            },
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
        let mut track_counter = 0;
        for track in &self.tracklist{
            let origin_loc = format!("/run/user/1000/gvfs/cdda:host={}/{}",&self.drive,&track.escape_debug());
            let target_loc = format!("{}/{}",self.temp_folder_name,&track);
            fs::copy(origin_loc, target_loc).unwrap();
            track_counter+=1;
            println!("Copied {:?} ({}/{})", &track, &track_counter, &self.tracklist.len());
        }
        self.toggle_eject_disc()
    }


    fn toggle_eject_disc(&self){
        self.drive_dev.toggle_eject().expect("Failed to toggle the Disk tray!");
    }


    fn combine_files(&self, tmp_file_name: String, tracklist: &Vec<String>){
        //!Combines the copied files using sox into `tmp.wav`
        let mut files:Vec<String> = vec![];
        for track in tracklist{
            files.push(format!("{}/{}", self.temp_folder_name,track.to_string()));
        }
        wav_concat::wav_concat(tracklist.to_vec(), format!("{}/{}",self.temp_folder_name, tmp_file_name));
        println!("Successfully combined files")
    }

    fn clean_tmp_folder(&self){
        for track in &self.tracklist{
            fs::remove_file(format!("{}/{}",self.temp_folder_name,track)).unwrap();
        }
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
        println!("Successfully converted to mp3")
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