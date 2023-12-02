### reepicheep
<img src="https://github.com/danielbeach/reepicheep/blob/main/imgs/reep.png" width="30">

This is a `Rust` based package to help with the management of
complex medicine (pill) management cycles.

`reepicheep` notifies a person(s) via `SMS` text message using a `twillio` account.
Many complex treatment plans for those with cancer and other problems
require various medications taken at all various times of the day,
and various days during long cycle periods.

It is very challenging to manage these pills via classic 
"pill dispensers," as even this can be challenging and
error prone when filling the pill dispensers and its easy
to forget with a busy life.

#### Configuration
See `meds.json` for the configuration of multiple meds.
It more or less looks like ...
```
{
    "number_of_cycles" : 8,
    "cycle_start_date": "2023-11-28",
    "lenght_of_cycles_in_days": 28,
    "meds": [
        {
            "med_name": "Acyclovir",
            "moring": "True",
            "evening": "True",
            "daily": "True",
            "cycle_days": []
        }, ...
    ]
}
```

Using `reepicheep` you can setup your cycles and
specifcy their usages and interavls and then use
the `SMS` notifications to help support your 
cycles to reduce stress and axiety about missing
or mistaking medications.

`reepicheep` currently supports sending morning and
evening notifications in the `central` timezone around
`7am` and `530pm`.


#### Twillio
You must have a `twillio` account and obtain the following creds,
which should be placed in a `.env` file in your root. You will of
course need the `from` phone number and the number you're sending
the message too.
```
TWILIO_AUTH_TOKEN="{}"
TWILIO_ACCOUNT_SID="{}"
TWILIO_PHONE_NUMBER="{}"
RECIPIENT_PHONE_NUMBER="{}"
```

#### SQLite
`Rust` works very well with `SQLite` and `reepicheep` uses `SQLite`
to easily track what medication notifications have been sent or not.

#### Build and Run
Simply `cargo run --release` or `cargo build ..` 

