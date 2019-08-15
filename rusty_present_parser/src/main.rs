use libm;
use std::env;
use std::error::Error;
use std::path::Path;
use std::process;
use std::process::Command;
use std::{ffi::OsStr, fs};

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
    allows_tearing: String, //6 Can use this maybe to detect it some kind of frame sync is enabled which will cause odd results
    #[serde(rename = "PresentMode")]
    present_mode: String, //7
    #[serde(rename = "Dropped")]
    dropped: String, //8 This is a string so we can handle an "Error" in the column, parse the string later and turn it into an u64 as long as it is not "Error"
    //#[serde(deserialize_with = "csv::invalid_option")] //Sometimes 'Error' is in this column, try to handle it
    //dropped: Option<u64>, //8
    #[serde(rename = "TimeInSeconds")]
    time_in_seconds: f64, //9
    #[serde(rename = "MsBetweenPresents")]
    ms_between_presents: Option<f64>, //10 May panic if this value is zero, should not be the case until the frame is dropped (need fix,skip, or fall back to MsBetweenPresents?)
    #[serde(rename = "MsBetweenDisplayChange")]
    ms_between_display_change: f64, //11
    #[serde(rename = "MsInPresentAPI")]
    ms_in_present_api: Option<f64>, //12
    #[serde(rename = "MsUntilRenderComplete")]
    ms_until_render_complete: Option<f64>, //13
    #[serde(rename = "MsUntilDisplayed")]
    //May panic if this value is zero, though it is not used in calculations yet
    ms_until_displayed: Option<f64>, //14
}

//https://docs.rs/csv/1.1.1/csv/

fn calculate_ranged_fps(_v: &[f64], _p: f64) -> f64 {
    let mut _some_percent_fps = 0.0;
    let mut _some_percent_size = libm::ceil(_v.len() as f64 * _p) as u64;
    //println!("LOCATION: {:}", _v.len() - _some_percent_size as usize + 1);
    //_some_percent_fps = libm::floor(1000.0 / _v[_v.len() - _some_percent_size as usize]);
    //_some_percent_fps = libm::floor(1000.0 / _v[_some_percent_size as usize + 1]);
    //_some_percent_fps = 1000.0 / _v[_some_percent_size as usize + 1];\
    _some_percent_fps = 1000.0 / _v[_v.len() - _some_percent_size as usize];
    _some_percent_fps
}

fn calculate_average_ranged_fps(_v: &[f64], _p: f64) -> f64 {
    let mut _ranged_size = libm::floor(_v.len() as f64 * _p) as usize;
    //println!("RANGED SIZE: {:?}", _ranged_size);
    let mut _total_frame_time = 0.0;
    for time in _v.iter().rev().take(_ranged_size) {
        _total_frame_time += *time;
    }
    //libm::floor(1000.0 / (_total_frame_time / _ranged_size as f64))
    1000.0 / (_total_frame_time / _ranged_size as f64)
}

fn calculate_median_fps(_v: &[f64]) -> f64 {
    //yes, normally when taking the median of an even set we average the two values
    //we want to use only real values that occured so will take the lower of the two values
    //if _v.len() % 2 == 0 { 1000.0 / _v[(_v.len() / 2) - 1] } else { libm::floor(1000.0 / _v[_v.len() / 2]) }
    if _v.len() % 2 == 0 {
        1000.0 / _v[(_v.len() / 2) - 1]
    } else {
        1000.0 / _v[_v.len() / 2]
    }
}

fn calculate_average_fps(_v: &[f64], _total_frame_time: f64) -> f64 {
    let mut _average_fps = 0.0;
    _average_fps = 1000.0 / (_total_frame_time / _v.len() as f64);
    _average_fps
}

fn percent_time_below_threshold(_v: &[f64], _threshold: f64) -> f64 {
    let count = _v.iter().filter(|&n| *n > _threshold).count();
    100.0 * (count as f64 / _v.len() as f64)
}

/// Calculates the jitter of the data set of frametimes
/// Jitter is defined as the total difference of the set
/// divided by the size of the set minus 1
/// # Examples
///
/// ```
/// let arg = vec![136.0,184.0,115.0,148.0,125.0];
/// let answer = Self::calculate_jitter(&arg);
///
/// assert_eq!(43.25, answer);
/// ```
fn calculate_jitter(_v: &[f64]) -> f64 {
    //probably need to use the original unsorted vectors here
    //for now just calling before sorting the vector
    let mut _total_difference = 0.0;
    for i in 1.._v.len() {
        _total_difference += libm::fabs(_v[i] - _v[i - 1]);
    }
    _total_difference / (_v.len() as f64 - 1.0)
}

fn process_csv(_path: String) -> Result<(), Box<Error>> {
    let mut rdr = csv::Reader::from_path(_path)?;
    //let mut rdr = csv::Reader::from_path("..\\data\\ThreeKingdoms_battle-0.csv")?;
    let mut _dataset_has_error = false;

    let mut _total_frame_time = 0.0;
    let mut _dropped_frames = 0;

    let mut _frame_times_vec = vec![];
    for result in rdr.deserialize() {
        let record: Present = result?;
        _total_frame_time += record.ms_between_display_change;
        if record.dropped != "Error" {
            // Handle an error that may be present in the data set
            _dropped_frames += record.dropped.parse::<u64>().unwrap();
        } else {
            _dataset_has_error = true;
        }
        //_dropped_frames += record.dropped.unwrap(); //This may crash when it gets to a column that has "Error" in it instead of a u64
        _frame_times_vec.push(record.ms_between_display_change);
        //println!("{:?}", record.ms_between_display_change);
    }

    //this is done before sorting the vector, need to fix this so it can be called anytime (sorted and unsorted copies?)
    println!(
        "Jitter before sorting: {:.2?} ms",
        calculate_jitter(&_frame_times_vec)
    );

    //need to sort frametimes
    _frame_times_vec.sort_by(|a, b| a.partial_cmp(b).unwrap());

    if _dataset_has_error {
        //Add color support here to highligh the error better? https://github.com/BurntSushi/termcolor
        println!("An error was detected in the dataset, results may be invalid!");
    }

    println!("Total frame time in ms: {:?}", _total_frame_time.to_owned());
    println!(
        "Total frame time in s: {:?}",
        _total_frame_time.to_owned() / 1000.0
    );
    println!("Total frames rendered: {:?}", _frame_times_vec.len());
    println!("Total dropped frames: {:?}", _dropped_frames);

    println!(
        "1% Low FPS: {:.2?}",
        calculate_ranged_fps(&_frame_times_vec, 0.01)
    );
    println!(
        "0.1% Low FPS: {:.2?}",
        calculate_ranged_fps(&_frame_times_vec, 0.001)
    );
    println!(
        "Average FPS: {:.2?}",
        calculate_average_fps(&_frame_times_vec, _total_frame_time)
    );
    println!(
        "Median FPS: {:.2?}",
        calculate_median_fps(&_frame_times_vec)
    );
    println!(
        "Average 1% FPS: {:.2?}",
        calculate_average_ranged_fps(&_frame_times_vec, 0.01)
    );
    println!(
        "Average 0.1% FPS: {:.2?}",
        calculate_average_ranged_fps(&_frame_times_vec, 0.001)
    );
    println!(
        "Median FPS: {:.2?}",
        calculate_median_fps(&_frame_times_vec)
    );
    println!(
        "Below 60 FPS: {:.2?}%",
        percent_time_below_threshold(&_frame_times_vec, 16.66)
    );
    println!(
        "Below 144 FPS: {:.2?}%",
        percent_time_below_threshold(&_frame_times_vec, 6.944)
    );
    println!(
        "Below 165 FPS: {:.2?}%",
        percent_time_below_threshold(&_frame_times_vec, 6.060)
    );
    println!(
        "Below 240 FPS: {:.2?}%",
        percent_time_below_threshold(&_frame_times_vec, 4.166)
    );
    println!();
    println!();

    Ok(())
}

fn main() -> std::io::Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        println!("A file or directory argument is needed to run... ");
        process::exit(1);
    }
    let _path = Path::new(&args[1]);
    let _path_test = fs::metadata(_path)?;
    if _path_test.is_file() {
        println!("Single file {:?} is input", _path);
        if let Err(err) = process_csv(_path.to_str().unwrap().to_string()) {
            //this may be a little dangerous, an Option is returned
            println!("error running process_csv: {}", err);
            process::exit(1);
        }
    }
    if _path_test.is_dir() {
        println!("Processing directory files in {:?}\n", _path);
        //since we got a directory, we need to process for now, all the csv data files that exist
        //walk the dir looking for csv data files process_csv for each file that exists
        //modified this code to implement this section: https://stackoverflow.com/a/51419126
        let mut result = vec![];
        for _dir_path in fs::read_dir(_path)? {
            let _dir_path = _dir_path?.path();
            if let Some("csv") = _dir_path.extension().and_then(OsStr::to_str) {
                result.push(_dir_path.to_owned());
            }
        }
        println!("# CSV FILES FOUND: {:?}\n", result.len());
        for csv in result {
            println!("WORKING FILE: {:?}", csv);
            if let Err(err) = process_csv(csv.to_str().unwrap().to_string()) {
                println!("error running process_csv: {}", err);
                process::exit(1);
            }
        }
    }
    let _ = Command::new("cmd.exe").arg("/c").arg("pause").status();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::{thread_rng, Rng};

    #[test]
    fn jitter_correct() {
        let v = vec![136.0, 184.0, 115.0, 148.0, 125.0];
        assert_eq!(43.25, calculate_jitter(&v));
    }

    #[test]
    fn below_threshold_count() {
        let v = vec![23.3, 12.2, 45.6, 16.6, 16.5];
        assert_eq!(40.0, percent_time_below_threshold(&v, 16.66));
    }

    #[test]
    fn median_odd_set() {
        let mut v = vec![2.3, 5.6, 1.0, 9.6, 12.3];
        v.sort_by(|a, b| a.partial_cmp(b).unwrap());
        assert_eq!(178.57142857142858, calculate_median_fps(&v));
    }

    #[test]
    fn correct_ranged_fps_zero_point_one_percent_low() {
        let mut rng = thread_rng();
        let mut numbers: Vec<f64> = (0..1000).map(|_| rng.gen_range(1.0, 101.0)).collect();
        numbers.sort_by(|a, b| a.partial_cmp(b).unwrap());
        numbers[999] = 16.66;
        assert_eq!(1000.0 / numbers[999], calculate_ranged_fps(&numbers, 0.001));
    }

    #[test]
    fn correct_average_ranged_fps_one_percent_low() {
        let mut rng = thread_rng();
        let mut numbers: Vec<f64> = (0..1000).map(|_| rng.gen_range(1.0, 101.0)).collect();
        numbers.sort_by(|a, b| a.partial_cmp(b).unwrap());

        //made a data set of 1000, sorted and then make sure that the last 10 are used for average 1% FPS calculation
        numbers[990] = 16.66;
        numbers[991] = 5.66;
        numbers[992] = 3.45;
        numbers[993] = 17.78;
        numbers[994] = 14.56;
        numbers[995] = 12.34;
        numbers[996] = 16.54;
        numbers[997] = 6.55;
        numbers[998] = 8.67;
        numbers[999] = 9.99;
        //112.2 Average Frametime: 11.22 FPS: 89.126559714795008912655971479501‬

        assert_eq!(
            89.12655971479501,
            calculate_average_ranged_fps(&numbers, 0.01)
        );
    }
}
