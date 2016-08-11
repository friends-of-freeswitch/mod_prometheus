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
use std::collections::HashMap;
use std::ops::Index;

use freeswitchrs::raw as fsr;
use freeswitchrs::mods::*; // This will get replaced with a mods prelude
use freeswitchrs::Status;
use freeswitchrs::raw::log_level::{INFO, WARNING};

use prometheus::{Registry, Counter, Gauge};

enum FSCounter {
    Heartbeats = 0,
    Sessions,
    SessionsAnswered,
    SessionsFailed,
    SessionsInbound,
    SessionsInboundAnswered,
    SessionsInboundFailed,
    SessionsOutbound,
    SessionsOutboundAnswered,
    SessionsOutboundFailed,
    Registrations,
    RegistrationAttempts,
    RegistrationFailures
}

enum FSGauge {
    SessionsActive,
    SessionsASR,
    RegistrationsActive
}

lazy_static! {
    static ref REGISTRY: Arc<Mutex<Registry>> = {
        Arc::new(Mutex::new(Registry::new("0.0.0.0".to_string(), 6780)))
    };
    static ref USER_COUNTERS: HashMap<String, Arc<Mutex<Counter>>> = {
        HashMap::new()
    };
    static ref USER_GAUGES: HashMap<String, Arc<Mutex<Gauge>>> = {
        HashMap::new()
    };
    static ref COUNTERS: [Arc<Mutex<Counter>>;13] = {[
        Arc::new(Mutex::new(Counter::new("freeswitch_heartbeats".to_string(),
                                         "FreeSWITCH heartbeat count".to_string()))),

        Arc::new(Mutex::new(prometheus::Counter::new("freeswitch_sessions".to_string(),
                                                     "FreeSWITCH Session Count".to_string()))),

        Arc::new(Mutex::new(prometheus::Counter::new("freeswitch_sessions_answered".to_string(),
                                                     "FreeSWITCH Answered Sessions Count".to_string()))),

        Arc::new(Mutex::new(prometheus::Counter::new("freeswitch_sessions_failed".to_string(),
                                                     "FreeSWITCH Failed Sessions Count".to_string()))),

        Arc::new(Mutex::new(prometheus::Counter::new("freeswitch_sessions_inbound".to_string(),
                                                     "FreeSWITCH Inbound Sessions Count".to_string()))),

        Arc::new(Mutex::new(prometheus::Counter::new("freeswitch_sessions_inbound_answered".to_string(),
                                                     "FreeSWITCH Answered Inbound Sessions Count".to_string()))),

        Arc::new(Mutex::new(prometheus::Counter::new("freeswitch_sessions_inbound_failed".to_string(),
                                                     "FreeSWITCH Failed Inbound Sessions Count".to_string()))),

        Arc::new(Mutex::new(prometheus::Counter::new("freeswitch_sessions_outbound".to_string(),
                                                     "FreeSWITCH Outbound Sessions Count".to_string()))),

        Arc::new(Mutex::new(prometheus::Counter::new("freeswitch_sessions_outbound_answered".to_string(),
                                                     "FreeSWITCH Answered Outbound Sessions Count".to_string()))),

        Arc::new(Mutex::new(prometheus::Counter::new("freeswitch_sessions_outbound_failed".to_string(),
                                                     "FreeSWITCH Failed Outbound Sessions Count".to_string()))),

        // Registration Metrics
        Arc::new(Mutex::new(prometheus::Counter::new("freeswitch_registrations".to_string(),
                                                     "FreeSWITCH Registration Count".to_string()))),

        Arc::new(Mutex::new(prometheus::Counter::new("freeswitch_registration_attempts".to_string(),
                                                     "FreeSWITCH Registration Attempts".to_string()))),

        Arc::new(Mutex::new(prometheus::Counter::new("freeswitch_registration_failures".to_string(),
                                                     "FreeSWITCH Registration Failures".to_string())))
    ]};
    static ref GAUGES: [Arc<Mutex<Gauge>>;3] = {[
        Arc::new(Mutex::new(prometheus::Gauge::new("freeswitch_sessions_active".to_string(),
                                                   "FreeSWITCH Active Sessions".to_string()))),

        Arc::new(Mutex::new(prometheus::Gauge::new("freeswitch_sessions_asr".to_string(),
                                                   "FreeSWITCH Sessions Answer Seizure Ratio".to_string()))),

        Arc::new(Mutex::new(prometheus::Gauge::new("freeswitch_registrations_active".to_string(),
                                                   "FreeSWITCH Active Registrations".to_string())))
    ]};
}

impl Index<FSCounter> for [Arc<Mutex<Counter>>] {
    type Output = Arc<Mutex<Counter>>;
    fn index(&self, idx: FSCounter) -> &Arc<Mutex<Counter>> {
        &self[idx as usize]
    }
}

impl Index<FSGauge> for [Arc<Mutex<Gauge>>] {
    type Output = Arc<Mutex<Gauge>>;
    fn index(&self, idx: FSGauge) -> &Arc<Mutex<Gauge>> {
        &self[idx as usize]
    }
}

fn prometheus_load(mod_int: &ModInterface) -> Status {
    let ref reg = *REGISTRY;
    // At some point we'll have to configure things ...
    //let xml = fsr::xml_open_cfg();
    Registry::start(&reg);
    {
        let mut r = reg.lock().unwrap();
        for c in COUNTERS.iter() {
            r.register_counter(c.clone());
        }
        for g in GAUGES.iter() {
            r.register_gauge(g.clone());
        }
    }
    // Heartbeat counts
    freeswitchrs::event_bind("mod_prometheus", fsr::event_types::HEARTBEAT, None, |_| {
        COUNTERS[FSCounter::Heartbeats].lock().unwrap().increment();
    });
    // New channel created
    freeswitchrs::event_bind("mod_prometheus", fsr::event_types::CHANNEL_CREATE, None, |e| {
        COUNTERS[FSCounter::Sessions].lock().unwrap().increment();
        GAUGES[FSGauge::SessionsActive].lock().unwrap().increment();
        if let Some(direction) = e.header("Call-Direction") {
            if direction == "inbound" {
                COUNTERS[FSCounter::SessionsInbound].lock().unwrap().increment();
            } else {
                let outbound = COUNTERS[FSCounter::SessionsOutbound].lock().unwrap().increment();
                let asr = COUNTERS[FSCounter::SessionsOutboundAnswered].lock().unwrap().value() / outbound;
                GAUGES[FSGauge::SessionsASR].lock().unwrap().set(asr);
            }
        } else {
            let b = e.body().unwrap_or(Cow::Borrowed("<No Body>"));
            unsafe { fslog!(WARNING, "Received channel create event with no call direction: {:?}\n", b); }
        }
    });

    // Channel answered
    freeswitchrs::event_bind("mod_prometheus", fsr::event_types::CHANNEL_ANSWER, None, |e| {
        COUNTERS[FSCounter::SessionsAnswered].lock().unwrap().increment();
        if let Some(direction) = e.header("Call-Direction") {
            if direction == "inbound" {
                COUNTERS[FSCounter::SessionsInboundAnswered].lock().unwrap().increment();
            } else {
                let answered = COUNTERS[FSCounter::SessionsOutboundAnswered].lock().unwrap().increment();
                let asr = answered / COUNTERS[FSCounter::SessionsOutbound].lock().unwrap().value();
                GAUGES[FSGauge::SessionsASR].lock().unwrap().set(asr);
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
                        COUNTERS[FSCounter::SessionsInboundFailed].lock().unwrap().increment();
                    } else {
                        COUNTERS[FSCounter::SessionsInboundFailed].lock().unwrap().increment();
                    }
                    COUNTERS[FSCounter::SessionsFailed].lock().unwrap().increment();
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
        GAUGES[FSGauge::SessionsActive].lock().unwrap().decrement();
    });

    // FIXME: Registrations are bound to be outdated on restart (registrations are in the db)
    // so we should fetch them on module load to get the counters initialized

    // Registration attempts
    freeswitchrs::event_bind("mod_prometheus", fsr::event_types::CUSTOM, Some("sofia::register_attempt"), |_| {
        COUNTERS[FSCounter::RegistrationAttempts].lock().unwrap().increment();
    });

    // Registration failures
    freeswitchrs::event_bind("mod_prometheus", fsr::event_types::CUSTOM, Some("sofia::register_failure"), |_| {
        COUNTERS[FSCounter::RegistrationFailures].lock().unwrap().increment();
    });

    // Registration counters
    freeswitchrs::event_bind("mod_prometheus", fsr::event_types::CUSTOM, Some("sofia::register"), |_| {
        COUNTERS[FSCounter::Registrations].lock().unwrap().increment();
        GAUGES[FSGauge::RegistrationsActive].lock().unwrap().increment();
    });
    freeswitchrs::event_bind("mod_prometheus", fsr::event_types::CUSTOM, Some("sofia::unregister"), |_| {
        GAUGES[FSGauge::RegistrationsActive].lock().unwrap().decrement();
    });
    freeswitchrs::event_bind("mod_prometheus", fsr::event_types::CUSTOM, Some("sofia::expire"), |_| {
        GAUGES[FSGauge::RegistrationsActive].lock().unwrap().decrement();
    });

    mod_int.add_raw_api("counter_increase", "Increase counter", "counter_increase", counter_increase_api);
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
