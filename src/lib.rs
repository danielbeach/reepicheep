use chrono::{NaiveDate, Timelike};
use chrono_tz::America::Chicago;
use serde::{Serialize, Deserialize};
use rusqlite::{Connection, Result, params};
use dotenv::dotenv;
use reqwest::{blocking::Client, Error, StatusCode};
use std::env;

#[derive(Serialize, Deserialize, Debug)]
pub struct MedicationPlan {
    number_of_cycles: u8,
    cycle_start_date: String,
    length_of_cycles_in_days: u8,
    meds: Vec<Medication>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Medication {
    med_name: String,
    morning: bool,
    evening: bool,
    daily: bool,
    cycle_days: Vec<u8>,
}

#[derive(Debug)]
pub struct CyclePlan {
    cycle_date: NaiveDate,
    morning_message_sent: bool,
    evening_message_sent: bool,
}

enum TimeOfDay {
    Morning,
    Evening,
}

#[derive(Serialize, Deserialize)]
struct SMSResponse {
    account_sid: Option<String>,
    api_version: String,
    body: String,
    date_created: String,
    date_sent: String,
    date_updated: String,
    direction: String,
    error_code: String,
    error_message: String,
    from: String,
    messaging_service_sid: String,
    num_media: String,
    num_segments: String,
    price: String,
    price_unit: String,
    sid: String,
    status: String,
    subresource_uris: SubresourceUris,
    to: String,
    uri: String,
}

#[derive(Serialize, Deserialize)]
struct SubresourceUris {
    all_time: String,
    today: String,
    yesterday: String,
    this_month: String,
    last_month: String,
    daily: String,
    monthly: String,
    yearly: String,
}

#[derive(Serialize, Deserialize)]
struct ErrorResponse {
    code: u16,
    message: String,
    more_info: String,
    status: u16
}


pub fn read_json_file(path: String) -> String {
    let file = std::fs::read_to_string(path).expect("Unable to read file");
    file
}

pub fn parse_json_to_struct(json_string: String) -> MedicationPlan {
    let plan: MedicationPlan = serde_json::from_str(&json_string).unwrap();
    plan
}

pub fn calculate_cycle_end_date(plan: &MedicationPlan) -> NaiveDate {
    let start_date = chrono::NaiveDate::parse_from_str(&plan.cycle_start_date, "%Y-%m-%d").unwrap();
    let end_date = start_date + chrono::Duration::days(plan.length_of_cycles_in_days as i64 * plan.number_of_cycles as i64);
    end_date
}

pub fn have_the_cycles_ended_or_not(end_date: NaiveDate) -> u8 {
    if end_date.and_hms_opt(0, 0, 0).unwrap() > chrono::offset::Utc::now().naive_utc() {
        0
    } else {
        1
    }
}

fn calculate_date_range_for_cycles(plan: &MedicationPlan) -> Vec<NaiveDate> {
    let start_date = chrono::NaiveDate::parse_from_str(&plan.cycle_start_date, "%Y-%m-%d").unwrap();
    let end_date = calculate_cycle_end_date(plan);
    println!("End date: {:?}", end_date);
    let mut date_range = Vec::new();
    let mut current_date = start_date;
    while current_date < end_date {
        date_range.push(current_date);
        current_date = current_date + chrono::Duration::days(1);
    }
    date_range
}

pub fn insert_cycle_dates_into_db(plan: &MedicationPlan) {
    let conn = Connection::open("meds.db").expect("Unable to open db");
    let date_range = calculate_date_range_for_cycles(plan);
    for date in date_range {
        conn.execute(
            "INSERT INTO cycles (cycle_date, morning_message_sent, evening_message_sent) VALUES (?1, ?2, ?3)",
            &[&date.to_string(), &0.to_string(), &0.to_string()],
        ).unwrap();
    }
    conn.close().unwrap();
}

fn update_morning_or_evening_as_sent(plan: &MedicationPlan, time_of_day: TimeOfDay) {
    let conn = Connection::open("meds.db").expect("Unable to open db");
    match time_of_day {
        TimeOfDay::Morning => conn.execute(
            "UPDATE cycles SET morning_message_sent = ?1 WHERE cycle_date = ?2",
            &[&1.to_string(), &chrono::offset::Utc::now().naive_utc().date().to_string()],
        ).unwrap(),
        TimeOfDay::Evening => conn.execute(
            "UPDATE cycles SET evening_message_sent = ?1 WHERE cycle_date = ?2",
            &[&1.to_string(), &chrono::offset::Utc::now().naive_utc().date().to_string()],
        ).unwrap(),
    };
    conn.close().unwrap();

}

pub fn create_sqlite_db() {
    let conn = Connection::open("meds.db").expect("Unable to create db");
    conn.execute("DROP TABLE IF EXISTS cycles;",[]).unwrap();
    conn.execute(
        "CREATE TABLE cycles (
            cycle_date DATE NOT NULL,
            morning_message_sent BOOLEAN,
            evening_message_sent BOOLEAN
        )",
        [],
    ).unwrap();
    conn.close().unwrap();
}

pub fn check_on_todays_messages() -> Result<Vec<CyclePlan>, rusqlite::Error> {
    let conn = Connection::open("meds.db").unwrap();
    let today = chrono::offset::Utc::now().naive_utc().date().to_string();

    let mut stmt = conn.prepare("SELECT cycle_date, morning_message_sent, evening_message_sent FROM cycles WHERE cycle_date = ?1").unwrap();
    let cycle_plans_iter = stmt.query_map(params![today], |row| {
        let cycle_date_str: String = row.get(0).unwrap();
        let cycle_date = NaiveDate::parse_from_str(&cycle_date_str, "%Y-%m-%d");
        Ok(CyclePlan {
            cycle_date: cycle_date.unwrap(),
            morning_message_sent: row.get(1).unwrap(),
            evening_message_sent: row.get(2).unwrap()
        })
    }).unwrap();

    let cycle_plans_vec = cycle_plans_iter.collect();
    return cycle_plans_vec;
}

pub fn check_todays_message_status(plan: &MedicationPlan) {
    let todays_status = check_on_todays_messages().unwrap();
    let current_time = chrono::offset::Utc::now().with_timezone(&Chicago).naive_utc().time();

    for status in todays_status {
        let hour = current_time.hour();
        let minute = current_time.minute();
        // print the current time and the status of the morning and evening messages
        println!("Current time: {:?}. Morning message sent: {:?}. Evening message sent: {:?}.", current_time, status.morning_message_sent, status.evening_message_sent);

        if !status.morning_message_sent && hour == 7 && minute < 6 {
            println!("Morning message has not been sent");
            let morning_meds = gather_meds(plan, TimeOfDay::Morning);
            println!("Morning meds: {:?}", morning_meds);
            let morning_message = basic_message_structure(morning_meds, TimeOfDay::Morning);
            send_message(morning_message);
            update_morning_or_evening_as_sent(plan, TimeOfDay::Morning);
        }

        if !status.evening_message_sent && (hour == 17 && minute >= 25) || (hour == 18 && minute <= 1) {
            println!("Evening message has not been sent");
            let evening_meds = gather_meds(plan, TimeOfDay::Evening);
            println!("Evening meds: {:?}", evening_meds);
            let evening_message = basic_message_structure(evening_meds, TimeOfDay::Evening);
            send_message(evening_message);
            update_morning_or_evening_as_sent(plan, TimeOfDay::Evening);
        }
    }
}

fn gather_meds(plan: &MedicationPlan, time_of_day: TimeOfDay) -> Vec<String> {
    let mut meds = Vec::new();
    let cycle_day = get_cycle_day(plan);

    for med in &plan.meds {
        let is_morning = med.morning && (med.daily || med.cycle_days.contains(&cycle_day));
        let is_evening = med.evening && (med.daily || med.cycle_days.contains(&cycle_day));

        match time_of_day {
            TimeOfDay::Morning if is_morning => meds.push(med.med_name.to_string()),
            TimeOfDay::Evening if is_evening => meds.push(med.med_name.to_string()),
            _ => (),
        }
    }

    meds
}

fn get_cycle_day(plan: &MedicationPlan) -> u8 {
    let start_date = chrono::NaiveDate::parse_from_str(&plan.cycle_start_date, "%Y-%m-%d").unwrap();
    let today = chrono::offset::Utc::now().naive_utc().date();
    let cycle_day = today.signed_duration_since(start_date).num_days() as u8;
    cycle_day
}

fn basic_message_structure (meds: Vec<String>, time_of_day: TimeOfDay) -> String {
    let mut message = String::new();
    match time_of_day {
        TimeOfDay::Morning => message.push_str("Good morning!"),
        TimeOfDay::Evening => message.push_str("Good evening!"),
    }
    message.push_str(" Please take your ");
    for med in meds {
        message.push_str(&med);
        message.push_str(", ");
    }
    message.pop();
    message.pop();
    message.push_str(".");
    message
}

fn handle_error(body: String) {
    let error_response: ErrorResponse = serde_json::from_str(&body).expect("Unable to deserialise JSON error response.");
    println!("SMS was not able to be sent because: {:?}.", error_response.message);
}

fn handle_success(body: String) {
    let sms_response: SMSResponse = serde_json::from_str(&body).expect("Unable to deserialise JSON success response.");
    println!("Your SMS with the body \"{:?}\".", sms_response.body);
}

fn send_message(sms_body: String) {
    let twilio_account_sid =
        env::var("TWILIO_ACCOUNT_SID").expect("Twilio Account SID could not be retrieved.");
    let twilio_auth_token =
        env::var("TWILIO_AUTH_TOKEN").expect("Twilio Auth Token could not be retrieved.");
    let twilio_phone_number =
        env::var("TWILIO_PHONE_NUMBER").expect("The Twilio phone number could not be retrieved.");
    let recipient_phone_number = env::var("RECIPIENT_PHONE_NUMBER")
        .expect("The recipient's phone number could not be retrieved.");
    let request_url =
        format!("https://api.twilio.com/2010-04-01/Accounts/{twilio_account_sid}/Messages.json");
        let client = Client::new();
        let request_params = [
            ("To", &recipient_phone_number),
            ("From", &twilio_phone_number),
            ("Body", &sms_body),
        ];
        let response = client
            .post(request_url)
            .basic_auth(twilio_account_sid, Some(twilio_auth_token))
            .form(&request_params)
            .send().unwrap();
    
        let status = response.status();
        let body = match response.text() {
            Ok(result) => result,
            Err(error) => panic!(
                "Problem extracting the JSON body content. Reason: {:?}",
                error
            ),
        };
        match status {
            StatusCode::BAD_REQUEST => handle_error(body),
            StatusCode::OK => handle_success(body),
            _ => println!("Received status code: {}", status),
        }
    
}