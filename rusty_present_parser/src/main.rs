use std::error::Error;
use std::io;
use std::process;
use libm;

use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct Present {
		#[serde(rename = "Application")]
		application: String, //0
		#[serde(rename = "ProcessID")]
		process_id: String, //1
		#[serde(rename = "SwapChainAddress")]
		swap_chain_address: String, //2
		#[serde(rename = "Runtime")]
		runtime: String, //3
		#[serde(rename = "SyncInterval")]
		sync_interval: u64, //4
		#[serde(rename = "PresentFlags")]
		present_flags: Option<u64>, //5
		#[serde(rename = "AllowsTearing")]
		allows_tearing: String, //6
		#[serde(rename = "PresentMode")]
		present_mode: String, //7
		#[serde(rename = "Dropped")]
		#[serde(deserialize_with = "csv::invalid_option")]
		dropped: Option<u64>, //8
		#[serde(rename = "TimeInSeconds")]
		time_in_seconds: f64, //9
		#[serde(rename = "MsBetweenPresents")]
		ms_between_presents: Option<f64>, //10
		#[serde(rename = "MsBetweenDisplayChange")]
		ms_between_display_change: f64, //11
		#[serde(rename = "MsInPresentAPI")]
		ms_in_present_api: Option<f64>, //12
		#[serde(rename = "MsUntilRenderComplete")]
		ms_until_render_complete: Option<f64>, //13
		#[serde(rename = "MsUntilDisplayed")]
		ms_until_displayed: Option<f64>, //14		
}

//https://docs.rs/csv/1.1.1/csv/

/*
fn example() -> Result<(), Box<dyn Error>> {
    //let mut rdr = csv::Reader::from_reader(io::stdin());
	let mut rdr = csv::Reader::from_path("..\\fortnite.csv")?;
    for result in rdr.deserialize() {
        // Notice that we need to provide a type hint for automatic
        // deserialization.
        let present: Present = result?;
		//let ms1 = result.MsBetweenPresents;
        println!("{:?}", present);
		//println!("{:?}", ms1);
    }
    Ok(())
}
*/

fn example() -> Result<(), Box<Error>> {
    //let mut rdr = csv::Reader::from_reader(io::stdin());
	//let mut rdr = csv::Reader::from_path("..\\data\\fortnite.csv")?;
	let mut rdr = csv::Reader::from_path("..\\data\\ThreeKingdoms_battle-0.csv")?;
	
	let mut _total_frame_time = 0.0;
	let mut _average_fps = 0.0;
	let mut _median_fps = 0.0;
	let mut _one_percent_fps = 0.0;
	let mut _point_one_percent_fps = 0.0;
	
	let mut _frame_times_vec = vec![];
	for result in rdr.deserialize() {        
		let record: Present = result?;
		_total_frame_time += record.ms_between_display_change;
		_frame_times_vec.push(record.ms_between_display_change);
		
        //println!("{:?}", record.ms_between_display_change);
    }
	
	//need to sort frametimes
	_frame_times_vec.sort_by(|a, b| a.partial_cmp(b).unwrap());
		
	//make this a function so that we can return percent low fps i.e. 0.1%, 1%, 3%, etc.
	let mut _one_percent_size = libm::ceil(_frame_times_vec.len() as f64 * 0.01) as u64;
	let mut _point_one_percent_size = libm::ceil(_frame_times_vec.len() as f64 * 0.001) as u64;
	_one_percent_fps = libm::floor(1000.0 / _frame_times_vec[_frame_times_vec.len() - _one_percent_size as usize]);
	_point_one_percent_fps = libm::floor(1000.0 / _frame_times_vec[_frame_times_vec.len() - _point_one_percent_size as usize]);
	
	if _frame_times_vec.len() % 2 == 0 {
		//if the set is even median is normally the mean of the two middle numbers but we want to see true numbers
		//that actually existed in the set so take the lowest of the two
		_median_fps = 1000.0 / _frame_times_vec[(_frame_times_vec.len() / 2) - 1];
	} else {
		_median_fps = libm::floor(1000.0 / _frame_times_vec[_frame_times_vec.len() / 2]);
	}
	
	_average_fps = 1000.0 / (_total_frame_time / _frame_times_vec.len() as f64);
	
	//one liner not working well here, doing it in a few lines above for now
	//_one_percent_fps = libm::floor(1000.0 / _frame_times_vec[_frame_times_vec.len() - libm::ceil(_frame_times_vec.len() * 0.01)])
	
	println!("Size of one percent data set: {:?}", _one_percent_size);
	println!("1% percent low FPS: {:?}", _one_percent_fps);
	println!("0.1% percent low FPS: {:?}", _point_one_percent_fps);
	println!("Total frame time in ms: {:?}", _total_frame_time.to_owned());
    println!("Size of data set: {:?}", _frame_times_vec.len());
	
	/*
	println!("Contents of FrameTimeVec:");
    for x in _frame_times_vec.iter() {
        println!("> {}", x);
    }
	*/
	
	/*
	_frame_times_vec.sort_by(|a, b| a.partial_cmp(b).unwrap());
		
	println!("Sorted contents of FrameTimeVec:");
    for x in _frame_times_vec.iter() {
        println!("> {}", x);
    }
	*/
	
	println!("Average FPS calculated: {:?}", _average_fps);
	println!("Median FPS calculated: {:?}", _median_fps);
	Ok(())
}

fn main() {
	if let Err(err) = example() {
		println!("error running example: {}", err);
        process::exit(1);
	}
}
