use std::error::Error;
//use std::io;
use libm;
use std::process;
//use std::fs;
use std::env;
use std::process::Command;

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

    let mut _total_frame_time = 0.0;
    let mut _dropped_frames = 0;

    let mut _frame_times_vec = vec![];
    for result in rdr.deserialize() {
        let record: Present = result?;
        _total_frame_time += record.ms_between_display_change;
        _dropped_frames += record.dropped.unwrap();
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

    Ok(())
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        println!("A file or directory argument is needed to run... ");
        process::exit(1);
    }
    let _path = &args[1];
    //https://doc.rust-lang.org/std/fs/struct.Metadata.html
    //need to handle directories here also, check if arg is file or directory
    //if file continue and proceess, if directory look for csv files and proceess

    if let Err(err) = process_csv(_path.to_string()) {
        println!("error running process_csv: {}", err);
        process::exit(1);
    }

    let _ = Command::new("cmd.exe").arg("/c").arg("pause").status();
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
        let mut numbers: Vec<f64> = (0..1000)
            .map(|_| {
                // 1 (inclusive) to 101 (exclusive)
                rng.gen_range(1.0, 101.0)
            })
            .collect();
        numbers.sort_by(|a, b| a.partial_cmp(b).unwrap());
        numbers[999] = 16.66;
        println!("Frametime to compare to: {:}", numbers[999]);
        assert_eq!(1000.0 / numbers[999], calculate_ranged_fps(&numbers, 0.001));
    }
}
