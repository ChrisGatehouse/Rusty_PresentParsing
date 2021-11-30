// Copyright © 2019 Chris Gatehouse
// This program is licensed under the "MIT License".
// Please see the file LICENSE in this distribution
// for license terms.

use libm;
use serde::Deserialize;
use std::{env, error::Error, ffi::OsStr, fs, path::Path, process, process::Command};

/*
struct CsvStandardResult {
    median_fps: f64,
    average_fps: f64,
    one_percent_low_fps: f64,
    point_one_percent_low_fps: f64,
    has_error: bool,
    filename: String,
    //has_frame_sync: bool,
}
*/

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
    allows_tearing: String, //6 Can use this maybe to detect if some kind of frame sync is enabled which will cause odd or invalid results i.e. max frametime locked to monitor refresh
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

// Calculate the standard deviation of the frametimes
fn standard_deviation(_v: &[f64], _total_frame_time: f64) -> f64 {
    let mut variance = 0f64;
    for i in 0.._v.len() {
        variance += f64::powf(_v[i] - average_frametime(_v, _total_frame_time), 2.0);
    }
    f64::sqrt(variance / _v.len() as f64)
}

/// Calculates the FPS at a given percentile
/// Expected to run on a sorted vector of frametimes
fn calculate_ranged_fps(_v: &[f64], _p: f64) -> f64 {
    let _some_percent_size = libm::ceil(_v.len() as f64 * _p) as u64;
    1000.0 / _v[_v.len() - _some_percent_size as usize]
}

/// Calculates the average ranged FPS in the dataset
/// i.e. a range being a percentile
/// Expected to run on a sorted vector of frametimes
fn calculate_average_ranged_fps(_v: &[f64], _p: f64) -> f64 {
    let _ranged_size = libm::floor(_v.len() as f64 * _p) as usize;
    let mut _total_frame_time = 0.0;
    for time in _v.iter().rev().take(_ranged_size) {
        _total_frame_time += *time;
    }
    1000.0 / (_total_frame_time / _ranged_size as f64)
}

/// Finds and returns the median FPS in the dataset
/// Expected to run on a sorted vector of frametimes
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

/// Finds and returns the median frametime in the dataset
/// Expected to run on a sorted vector of frametimes
fn median_frametime(_v: &[f64]) -> f64 {
    if _v.len() % 2 == 0 {
        _v[(_v.len() / 2) - 1]
    } else {
        _v[_v.len() / 2]
    }
}

/// Calculates the maximum fps of the data set
/// Expected to run on a sorted vector of frametimes
fn calculate_max_fps(_v: &[f64]) -> f64 {
    1000.0 / _v[0]
}

/// Calculates the minimum fps of the data set
/// Expected to run on a sorted vector of frametimes
fn calculate_min_fps(_v: &[f64]) -> f64 {
    1000.0 / _v[_v.len() - 1]
}

/// Calculates the average FPS of the dataset
fn calculate_average_fps(_v: &[f64], _total_frame_time: f64) -> f64 {
    1000.0 / (_total_frame_time / _v.len() as f64)
}

/// Calculate the average frametime of the dataset
fn average_frametime(_v: &[f64], _total_frame_time: f64) -> f64 {
    _total_frame_time / _v.len() as f64
}

/// Calculates the percentage of time below a given FPS
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
    //for now just calling this before sorting the vector in fn process_csv
    let mut _total_difference = 0.0;
    for i in 1.._v.len() {
        _total_difference += libm::fabs(_v[i] - _v[i - 1]);
    }
    _total_difference / (_v.len() as f64 - 1.0)
}

/// This function handles the processing of the csv files and displays the data that was processes
/// The reader was adapted from examples found on burnsushi https://blog.burntsushi.net/csv/
/// # Arguments
///
/// * `_path` - A string that is the path to the csv file to be parsed
///
fn process_csv(_path: String) -> Result<(), Box<dyn Error>> {
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
            // Handle an error that may be present in the data set and log that it happened
            _dropped_frames += record.dropped.parse::<u64>().unwrap();
        } else {
            _dataset_has_error = true;
        }
        //_dropped_frames += record.dropped.unwrap(); //This may crash when it gets to a column that has "Error" in it instead of a u64
        _frame_times_vec.push(record.ms_between_display_change);
    }

    //this is done before sorting the vector, need to fix this so it can be called anytime (sorted and unsorted copies?)
    println!("Jitter: {:.2?}ms", calculate_jitter(&_frame_times_vec));
    println!(
        "Standard deviation: {:.2?}ms",
        standard_deviation(&_frame_times_vec, _total_frame_time)
    );

    //need to sort frametimes
    _frame_times_vec.sort_by(|a, b| a.partial_cmp(b).unwrap());

    if _dataset_has_error {
        //Add color support here to highligh the error better? https://github.com/BurntSushi/termcolor
        println!("An error was detected in the dataset, results may be invalid!");
    }

    println!("Total frame time: {:?}ms", _total_frame_time.to_owned());
    println!(
        "Total frame time: {:?}s",
        _total_frame_time.to_owned() / 1000.0
    );
    println!("Total frames rendered: {:?}", _frame_times_vec.len());
    println!("Total dropped frames: {:?}", _dropped_frames);

    println!(
        "Median FPS: \t  {:.2?}",
        calculate_median_fps(&_frame_times_vec)
    );
    println!(
        "1% Low FPS: \t  {:.2?}",
        calculate_ranged_fps(&_frame_times_vec, 0.01)
    );
    println!(
        "0.1% Low FPS: \t  {:.2?}",
        calculate_ranged_fps(&_frame_times_vec, 0.001)
    );
    println!(
        "Avg. FPS: \t  {:.2?}",
        calculate_average_fps(&_frame_times_vec, _total_frame_time)
    );
    println!(
        "Avg. 1% FPS: \t  {:.2?}",
        calculate_average_ranged_fps(&_frame_times_vec, 0.01)
    );
    println!(
        "Avg. 0.1% FPS: \t  {:.2?}",
        calculate_average_ranged_fps(&_frame_times_vec, 0.001)
    );
    println!("Max FPS: \t  {:.2?}", calculate_max_fps(&_frame_times_vec));
    println!("Min FPS: \t  {:.2?}", calculate_min_fps(&_frame_times_vec));
    println!(
        "Median Frametime: {:.2?}ms",
        median_frametime(&_frame_times_vec)
    );
    println!(
        "Avg. Frametime:\t  {:.2?}ms",
        average_frametime(&_frame_times_vec, _total_frame_time)
    );
    println!(
        "Below 60 FPS: \t  {:.2?}%",
        percent_time_below_threshold(&_frame_times_vec, 16.66) //ms equal to 60 FPS
    );
    println!(
        "Below 144 FPS: \t  {:.2?}%",
        percent_time_below_threshold(&_frame_times_vec, 6.944) //ms equal to 144 FPS
    );
    println!(
        "Below 165 FPS: \t  {:.2?}%",
        percent_time_below_threshold(&_frame_times_vec, 6.060) //ms equal to 165 FPS
    );
    println!(
        "Below 240 FPS: \t  {:.2?}%",
        percent_time_below_threshold(&_frame_times_vec, 4.166) //ms equal to 240 FPS
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
        println!("CSV FILES FOUND: {:?}\n", result.len());
        for csv in result {
            println!("WORKING ON FILE: {:?}", csv);
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
        //Test will check that given a defined set the result matches what is expected for that set
        let v = vec![136.0, 184.0, 115.0, 148.0, 125.0];
        assert_eq!(43.25, calculate_jitter(&v));
    }

    #[test]
    fn below_threshold_count() {
        //Test will check that given a defined set and threshold what is returned is the correct
        //percentage of values below the given threshold
        let v = vec![23.3, 12.2, 45.6, 16.6, 16.5];
        assert_eq!(40.0, percent_time_below_threshold(&v, 16.66));
    }

    #[test]
    fn median_odd_set() {
        //Test will check that a correct median value (in FPS) for an odd set is returned
        let mut v = vec![2.3, 5.6, 1.0, 9.6, 12.3];
        v.sort_by(|a, b| a.partial_cmp(b).unwrap());
        assert_eq!(178.57142857142858, calculate_median_fps(&v));
    }

    #[test]
    fn median_even_set() {
        //Test will check that a correct median value (in FPS) for an even set is returned
        //The correct value in our case is the bottom of the two middle values not the average
        let mut v = vec![2.3, 5.6, 1.0, 9.6, 12.3, 4.8];
        v.sort_by(|a, b| a.partial_cmp(b).unwrap());
        assert_eq!(208.33333333333334, calculate_median_fps(&v));
    }

    #[test]
    fn correct_avg_fps() {
        let v = vec![2.3, 5.6, 1.0, 9.6, 12.3, 4.8];
        assert_eq!(
            168.53932584269663,
            calculate_average_fps(&v, v.iter().sum::<f64>() as f64)
        );
    }

    #[test]
    fn correct_avg_frametime() {
        let v = vec![2.3, 5.6, 1.0, 9.6, 12.3, 4.8];
        assert_eq!(
            5.933333333333334,
            average_frametime(&v, v.iter().sum::<f64>() as f64)
        );
    }

    #[test]
    fn correct_ranged_fps_zero_point_one_percent_low() {
        //Test will generate a dataset of 1000 and then fill the 1 percent position with a defined value
        //The result that returns should exactly match the defined value in FPS
        let mut rng = thread_rng();
        let mut mock_frametimes_ms: Vec<f64> =
            (0..1000).map(|_| rng.gen_range(1.0, 101.0)).collect();
        mock_frametimes_ms.sort_by(|a, b| a.partial_cmp(b).unwrap());
        mock_frametimes_ms[999] = 16.66;
        assert_eq!(
            1000.0 / mock_frametimes_ms[999],
            calculate_ranged_fps(&mock_frametimes_ms, 0.001)
        );
    }

    #[test]
    fn correct_average_ranged_fps_one_percent_low() {
        //Test will generate a dataset of 1000 and then fill the last 1 percent with defined values
        //The result that returns should exactly match a value calculated by hand from the defined values
        let mut rng = thread_rng();
        let mut mock_frametimes_ms: Vec<f64> =
            (0..1000).map(|_| rng.gen_range(1.0, 101.0)).collect();
        mock_frametimes_ms.sort_by(|a, b| a.partial_cmp(b).unwrap());

        //made a data set of 1000, sorted and then make sure that the last 10 are used for average 1% FPS calculation
        mock_frametimes_ms[990] = 16.66;
        mock_frametimes_ms[991] = 5.66;
        mock_frametimes_ms[992] = 3.45;
        mock_frametimes_ms[993] = 17.78;
        mock_frametimes_ms[994] = 14.56;
        mock_frametimes_ms[995] = 12.34;
        mock_frametimes_ms[996] = 16.54;
        mock_frametimes_ms[997] = 6.55;
        mock_frametimes_ms[998] = 8.67;
        mock_frametimes_ms[999] = 9.99;
        //112.2 Average Frametime: 11.22 FPS: 89.126559714795008912655971479501‬
        assert_eq!(
            89.12655971479501,
            calculate_average_ranged_fps(&mock_frametimes_ms, 0.01)
        );
    }
}
