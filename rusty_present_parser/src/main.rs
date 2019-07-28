use std::error::Error;
use std::io;
use std::process;

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
	let mut rdr = csv::Reader::from_path("..\\fortnite.csv")?;
	let mut _totalFrameTime = 0.0;
	for result in rdr.deserialize() {        
		let record: Present = result?;
		_totalFrameTime += record.MsBetweenDisplayChange;
        //println!("{:?}", record.MsBetweenDisplayChange);
    }
	println!("{:?}", _totalFrameTime.to_owned());
    Ok(())
}

fn main() {
	if let Err(err) = example() {
		println!("error running example: {}", err);
        process::exit(1);
	}
}
