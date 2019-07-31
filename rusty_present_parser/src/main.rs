use std::error::Error;
use std::io;
use std::process;
use libm;

use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct Present {
		Application: String, //0
		ProcessID: String, //1
		SwapChainAddress: String, //2
		Runtime: String, //3
		SyncInterval: u64, //4
		PresentFlags: Option<u64>, //5
		AllowsTearing: String, //6
		PresentMode: String, //7
		#[serde(deserialize_with = "csv::invalid_option")]
		Dropped: Option<u64>, //8
		TimeInSeconds: f64, //9
		MsBetweenPresents: Option<f64>, //10
		MsBetweenDisplayChange: f64, //11
		MsInPresentAPI: Option<f64>, //12
		MsUntilRenderComplete: Option<f64>, //13
		MsUntilDisplayed: Option<f64>, //14		
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
	
	let mut _totalFrameTime = 0.0;
	let mut _averageFPS = 0.0;
	let mut _median_FPS = 0.0;
	let mut _one_percent_fps = 0.0;
	let mut _point_one_percent_fps = 0.0;
	
	let mut _frameTimesVec = vec![];
	for result in rdr.deserialize() {        
		let record: Present = result?;
		_totalFrameTime += record.MsBetweenDisplayChange;
		_frameTimesVec.push(record.MsBetweenDisplayChange);
		
        //println!("{:?}", record.MsBetweenDisplayChange);
    }
	
	//need to sort frametimes
	_frameTimesVec.sort_by(|a, b| a.partial_cmp(b).unwrap());
		
	//make this a function so that we can return percent low fps i.e. 0.1%, 1%, 3%, etc.
	let mut _one_percent_size = libm::ceil(_frameTimesVec.len() as f64 * 0.01) as u64;
	let mut _point_one_percent_size = libm::ceil(_frameTimesVec.len() as f64 * 0.001) as u64;
	_one_percent_fps = libm::floor(1000.0 / _frameTimesVec[_frameTimesVec.len() - _one_percent_size as usize]);
	_point_one_percent_fps = libm::floor(1000.0 / _frameTimesVec[_frameTimesVec.len() - _point_one_percent_size as usize]);
	
	if _frameTimesVec.len() % 2 == 0 {
		//if the set is even median is normally the mean of the two middle numbers but we want to see true numbers
		//that actually existed in the set so take the lowest of the two
		_median_FPS = 1000.0 / _frameTimesVec[(_frameTimesVec.len() / 2) - 1];
	} else {
		_median_FPS = libm::floor(1000.0 / _frameTimesVec[_frameTimesVec.len() / 2]);
	}
	
	_averageFPS = 1000.0 / (_totalFrameTime / _frameTimesVec.len() as f64);
	
	//one liner not working well here, doing it in a few lines above for now
	//_one_percent_fps = libm::floor(1000.0 / _frameTimesVec[_frameTimesVec.len() - libm::ceil(_frameTimesVec.len() * 0.01)])
	
	println!("Size of one percent data set: {:?}", _one_percent_size);
	println!("1% percent low FPS: {:?}", _one_percent_fps);
	println!("0.1% percent low FPS: {:?}", _point_one_percent_fps);
	println!("Total frame time in ms: {:?}", _totalFrameTime.to_owned());
    println!("Size of data set: {:?}", _frameTimesVec.len());
	
	/*
	println!("Contents of FrameTimeVec:");
    for x in _frameTimesVec.iter() {
        println!("> {}", x);
    }
	*/
	
	/*
	_frameTimesVec.sort_by(|a, b| a.partial_cmp(b).unwrap());
		
	println!("Sorted contents of FrameTimeVec:");
    for x in _frameTimesVec.iter() {
        println!("> {}", x);
    }
	*/
	
	println!("Average FPS calculated: {:?}", _averageFPS);
	println!("Median FPS calculated: {:?}", _median_FPS);
	Ok(())
}

fn main() {
	if let Err(err) = example() {
		println!("error running example: {}", err);
        process::exit(1);
	}
}
