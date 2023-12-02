use chrono::NaiveDate;
use reepicheep::*;
use dotenv::dotenv;

fn main() {
    dotenv().ok();
    create_sqlite_db();
    let config_path = "meds.json";
    let config = read_json_file(config_path.to_string());
    println!("Finished load med plan from json file");

    let plan = parse_json_to_struct(config);
    println!("Finished parsing json to struct");

    insert_cycle_dates_into_db(&plan);
    println!("Finished inserting cycle dates into db");
    println!("starting to run infinite loop until the end of the cycle is reached");
    loop {

        let end_date: NaiveDate = calculate_cycle_end_date(&plan);
        let have_the_cycles_ended = have_the_cycles_ended_or_not(end_date);
        if have_the_cycles_ended == 1 {
            println!("The cycle has ended");
            break;
        }

        check_todays_message_status(&plan);

        std::thread::sleep(std::time::Duration::from_secs(5 * 60));
    }
}
