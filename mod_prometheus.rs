// TODO:
// - Refactor code to avoid using so many static globals and hide the ugliness
//   of Arc<Mutex<Counter|Gauge>>>
// - Make bindaddr configurable
// - Initialize counters/gauges to current values on module load
//   using switch_core_session_count(), switch_core_session_ctl() etc
// - Allow configuring metrics that can be later references the dialplan
// - Add dimensions to metrics (e.g inbound per profile)
// - Add error metrics (based on log errors/warnings)
#[macro_use]
extern crate lazy_static;

#[macro_use]
extern crate freeswitchrs;
extern crate prometheus;
extern crate libc;

use std::sync::{Arc, Mutex};
use std::borrow::Cow;

use freeswitchrs::raw as fsr;
use freeswitchrs::mods::*; // This will get replaced with a mods prelude
use freeswitchrs::Status;
use freeswitchrs::raw::log_level::{INFO, WARNING};

use prometheus::{Registry, Counter, Gauge};

lazy_static! {
    static ref REGISTRY: Arc<Mutex<Registry>> = {
        Arc::new(Mutex::new(prometheus::Registry::new("0.0.0.0".to_string(), 6780)))
    };
    static ref METRIC_HEARTBEAT_COUNT: Arc<Mutex<Counter>> = {
        Arc::new(Mutex::new(prometheus::Counter::new("freeswitch_heartbeat_count".to_string(),
                                                     "FreeSWITCH heartbeat count".to_string())))
    };
    static ref METRIC_SESSION_COUNT: Arc<Mutex<Counter>> = {
        Arc::new(Mutex::new(prometheus::Counter::new("freeswitch_session_count".to_string(),
                                                     "FreeSWITCH Session Count".to_string())))
    };
    static ref METRIC_SESSIONS_ANSWERED: Arc<Mutex<Counter>> = {
        Arc::new(Mutex::new(prometheus::Counter::new("freeswitch_sessions_answered".to_string(),
                                                   "FreeSWITCH Answered Sessions Count".to_string())))
    };
    static ref METRIC_SESSIONS_FAILED: Arc<Mutex<Counter>> = {
        Arc::new(Mutex::new(prometheus::Counter::new("freeswitch_sessions_failed".to_string(),
                                                     "FreeSWITCH Failed Sessions Count".to_string())))
    };
    static ref METRIC_SESSIONS_INBOUND: Arc<Mutex<Counter>> = {
        Arc::new(Mutex::new(prometheus::Counter::new("freeswitch_sessions_inbound".to_string(),
                                                   "FreeSWITCH Inbound Sessions Count".to_string())))
    };
    static ref METRIC_SESSIONS_INBOUND_ANSWERED: Arc<Mutex<Counter>> = {
        Arc::new(Mutex::new(prometheus::Counter::new("freeswitch_sessions_inbound_answered".to_string(),
                                                   "FreeSWITCH Answered Inbound Sessions Count".to_string())))
    };
    static ref METRIC_SESSIONS_INBOUND_FAILED: Arc<Mutex<Counter>> = {
        Arc::new(Mutex::new(prometheus::Counter::new("freeswitch_sessions_inbound_failed".to_string(),
                                                   "FreeSWITCH Failed Inbound Sessions Count".to_string())))
    };
    static ref METRIC_SESSIONS_OUTBOUND: Arc<Mutex<Counter>> = {
        Arc::new(Mutex::new(prometheus::Counter::new("freeswitch_sessions_outbound".to_string(),
                                                   "FreeSWITCH Outbound Sessions Count".to_string())))
    };
    static ref METRIC_SESSIONS_OUTBOUND_ANSWERED: Arc<Mutex<Counter>> = {
        Arc::new(Mutex::new(prometheus::Counter::new("freeswitch_sessions_outbound_answered".to_string(),
                                                   "FreeSWITCH Answered Outbound Sessions Count".to_string())))
    };
    static ref METRIC_SESSIONS_OUTBOUND_FAILED : Arc<Mutex<Counter>> = {
        Arc::new(Mutex::new(prometheus::Counter::new("freeswitch_sessions_outbound_failed".to_string(),
                                                   "FreeSWITCH Failed Outbound Sessions Count".to_string())))
    };
    static ref METRIC_SESSIONS_ACTIVE: Arc<Mutex<Gauge>> = {
        Arc::new(Mutex::new(prometheus::Gauge::new("freeswitch_sessions_active".to_string(),
                                                   "FreeSWITCH Active Sessions".to_string())))
    };
    static ref METRIC_SESSIONS_ASR: Arc<Mutex<Gauge>> = {
        Arc::new(Mutex::new(prometheus::Gauge::new("freeswitch_sessions_asr".to_string(),
                                                   "FreeSWITCH Answer Seizure Ratio".to_string())))
    };

    // Registration Metrics
    static ref METRIC_REGISTRATION_COUNT: Arc<Mutex<Counter>> = {
        Arc::new(Mutex::new(prometheus::Counter::new("freeswitch_registration_count".to_string(),
                                                   "FreeSWITCH Registration Count".to_string())))
    };
    static ref METRIC_REGISTRATION_ATTEMPTS: Arc<Mutex<Counter>> = {
        Arc::new(Mutex::new(prometheus::Counter::new("freeswitch_registration_attempts".to_string(),
                                                   "FreeSWITCH Registration Attempts".to_string())))
    };
    static ref METRIC_REGISTRATION_FAILURES: Arc<Mutex<Counter>> = {
        Arc::new(Mutex::new(prometheus::Counter::new("freeswitch_registration_failures".to_string(),
                                                   "FreeSWITCH Registration Failures".to_string())))
    };
    static ref METRIC_REGISTRATIONS_ACTIVE: Arc<Mutex<Gauge>> = {
        Arc::new(Mutex::new(prometheus::Gauge::new("freeswitch_registrations_active".to_string(),
                                                   "FreeSWITCH Active Registrations".to_string())))
    };
}

fn prometheus_load(mod_int: &ModInterface) -> Status {
    mod_int.add_raw_api("counter_increase", "Increase counter", "counter_increase", counter_increase_api);

    let ref reg = *REGISTRY;

    // At some point we'll have to configure things ...
    //let xml = fsr::xml_open_cfg();
    Registry::start(&reg);
    {
        let mut r = reg.lock().unwrap();

        // Session metrics
        r.register_counter((*METRIC_HEARTBEAT_COUNT).clone());
        r.register_counter((*METRIC_SESSION_COUNT).clone());
        r.register_counter((*METRIC_SESSIONS_FAILED).clone());
        r.register_counter((*METRIC_SESSIONS_ANSWERED).clone());
        r.register_counter((*METRIC_SESSIONS_INBOUND).clone());
        r.register_counter((*METRIC_SESSIONS_INBOUND_ANSWERED).clone());
        r.register_counter((*METRIC_SESSIONS_INBOUND_FAILED).clone());
        r.register_counter((*METRIC_SESSIONS_OUTBOUND).clone());
        r.register_counter((*METRIC_SESSIONS_OUTBOUND_ANSWERED).clone());
        r.register_counter((*METRIC_SESSIONS_OUTBOUND_FAILED).clone());
        r.register_gauge((*METRIC_SESSIONS_ACTIVE).clone());
        r.register_gauge((*METRIC_SESSIONS_ASR).clone());

        // Registration metrics
        r.register_counter((*METRIC_REGISTRATION_COUNT).clone());
        r.register_counter((*METRIC_REGISTRATION_ATTEMPTS).clone());
        r.register_counter((*METRIC_REGISTRATION_FAILURES).clone());
        r.register_gauge((*METRIC_REGISTRATIONS_ACTIVE).clone());
    }

    // Heartbeat counts
    freeswitchrs::event_bind("mod_prometheus", fsr::event_types::HEARTBEAT, None, |_| {
        METRIC_HEARTBEAT_COUNT.lock().unwrap().increment();
    });

    // New channel created
    freeswitchrs::event_bind("mod_prometheus", fsr::event_types::CHANNEL_CREATE, None, |e| {
        METRIC_SESSION_COUNT.lock().unwrap().increment();
        METRIC_SESSIONS_ACTIVE.lock().unwrap().increment();
        if let Some(direction) = e.header("Call-Direction") {
            if direction == "inbound" {
                METRIC_SESSIONS_INBOUND.lock().unwrap().increment();
            } else {
                let outbound = METRIC_SESSIONS_OUTBOUND.lock().unwrap().increment();
                let asr = METRIC_SESSIONS_OUTBOUND_ANSWERED.lock().unwrap().value() / outbound;
                METRIC_SESSIONS_ASR.lock().unwrap().set(asr);
            }
        } else {
            let b = e.body().unwrap_or(Cow::Borrowed("<No Body>"));
            unsafe { fslog!(WARNING, "Received channel create event with no call direction: {:?}\n", b); }
        }
    });

    // Channel answered
    freeswitchrs::event_bind("mod_prometheus", fsr::event_types::CHANNEL_ANSWER, None, |e| {
        METRIC_SESSIONS_ANSWERED.lock().unwrap().increment();
        if let Some(direction) = e.header("Call-Direction") {
            if direction == "inbound" {
                METRIC_SESSIONS_INBOUND_ANSWERED.lock().unwrap().increment();
            } else {
                let answered = METRIC_SESSIONS_OUTBOUND_ANSWERED.lock().unwrap().increment();
                let asr = answered / METRIC_SESSIONS_OUTBOUND.lock().unwrap().value();
                METRIC_SESSIONS_ASR.lock().unwrap().set(asr);
            }
        } else {
            let b = e.body().unwrap_or(Cow::Borrowed("<No Body>"));
            unsafe { fslog!(WARNING, "Received channel answer event with no call direction: {:?}\n", b); }
        }
    });

    // Channel hangup
    freeswitchrs::event_bind("mod_prometheus", fsr::event_types::CHANNEL_HANGUP, None, |e| {
        if let Some(answer) = e.header("Caller-Channel-Answered-Time") {
            let parsed_time = answer.parse::<i64>();
            if parsed_time.is_ok() && parsed_time.unwrap() == 0 as i64 {
                if let Some(direction) = e.header("Call-Direction") {
                    if direction == "inbound" {
                        METRIC_SESSIONS_INBOUND_FAILED.lock().unwrap().increment();
                    } else {
                        METRIC_SESSIONS_OUTBOUND_FAILED.lock().unwrap().increment();
                    }
                    METRIC_SESSIONS_FAILED.lock().unwrap().increment();
                } else {
                    let b = e.body().unwrap_or(Cow::Borrowed("<No Body>"));
                    unsafe { fslog!(WARNING, "Received channel hangup event with no call direction: {:?}\n", b); }
                }
            }
        } else {
            let b = e.body().unwrap_or(Cow::Borrowed("<No Body>"));
            unsafe { fslog!(WARNING, "Received channel hangup event with no call answer time information: {:?}\n", b); }
        }
    });

    // Channel destroyed
    freeswitchrs::event_bind("mod_prometheus", fsr::event_types::CHANNEL_DESTROY, None, |_| {
        METRIC_SESSIONS_ACTIVE.lock().unwrap().decrement();
    });

    // FIXME: Registrations are bound to be outdated on restart (registrations are in the db)
    // so we should fetch them on module load to get the counters initialized

    // Registration attempts
    freeswitchrs::event_bind("mod_prometheus", fsr::event_types::CUSTOM, Some("sofia::register_attempt"), |_| {
        METRIC_REGISTRATION_ATTEMPTS.lock().unwrap().increment();
    });

    // Registration failures
    freeswitchrs::event_bind("mod_prometheus", fsr::event_types::CUSTOM, Some("sofia::register_failure"), |_| {
        METRIC_REGISTRATION_FAILURES.lock().unwrap().increment();
    });

    // Registration counters
    freeswitchrs::event_bind("mod_prometheus", fsr::event_types::CUSTOM, Some("sofia::register"), |_| {
        METRIC_REGISTRATION_COUNT.lock().unwrap().increment();
        METRIC_REGISTRATIONS_ACTIVE.lock().unwrap().increment();
    });
    freeswitchrs::event_bind("mod_prometheus", fsr::event_types::CUSTOM, Some("sofia::unregister"), |_| {
        METRIC_REGISTRATIONS_ACTIVE.lock().unwrap().decrement();
    });
    freeswitchrs::event_bind("mod_prometheus", fsr::event_types::CUSTOM, Some("sofia::expire"), |_| {
        METRIC_REGISTRATIONS_ACTIVE.lock().unwrap().decrement();
    });


    unsafe {
        fslog!(INFO, "Loaded Prometheus Metrics Module{}", "");
    }
    Ok(())
}

fn prometheus_runtime() -> Status {
    //let ref reg = *REGISTRY;
    //loop {
    //}
    Err(fsr::status::TERM)
}

fn prometheus_unload() -> Status {
    let ref reg = *REGISTRY;
    Registry::stop(&reg);
    std::mem::drop(reg);
    Ok(())
}

static MOD_PROMETHEUS_DEF: ModDefinition = ModDefinition {
    name: "mod_prometheus",
    load: prometheus_load,
    runtime: Some(prometheus_runtime),
    shutdown: Some(prometheus_unload)
};

freeswitch_export_mod!(libmod_prometheus_module_interface, MOD_PROMETHEUS_DEF);

#[allow(unused_variables)]
unsafe extern "C" fn counter_increase_api(cmd: *const std::os::raw::c_char,
                                          session: *mut fsr::core_session,
                                          stream: *mut fsr::stream_handle)
                                          -> fsr::status {
    (*stream).write_function.unwrap()(stream, fsr::str_to_ptr("OK"));
    fsr::status::SUCCESS
}
