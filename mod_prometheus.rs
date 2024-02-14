// TODO:
// - Macro to bind event and store id? Ideally we should have the FS core remember the
//   events this module registered and the core can then unregister them, the same way
//   it works for module applications and APIs
// - Refactor code to avoid using so many static globals and hide the ugliness
//   of Arc<Mutex<Counter|Gauge>>>
// - Make bindaddr configurable
// - Initialize counters/gauges to current values on module load
//   using switch_core_session_count(), switch_core_session_ctl() etc
// - Allow configuring metrics that can be later references the dialplan
// - Add dimensions to metrics (e.g inbound per profile)
// - Add error metrics (based on log errors/warnings)
// - Add dialplan app, so if a gauge increased is associated with a session
//   it can be auto-decremented when the session is destroyed
// - Add label support
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
use freeswitchrs::raw::log_level::{DEBUG, INFO, WARNING, ERROR};

use prometheus::{Registry, Counter, Gauge};

// Ugh, note that these counter/gauge index values must map to the index
// in the COUNTERS/GAUGES globals. There is probably a less error-prone way
// to do this, but as of today it seems one can't iterate over enums in rust
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
    RegistrationFailures,
    SessionsOutboundCallDurationTotal,
}

enum FSGauge {
    SessionsActive,
    SessionsASR,
    RegistrationsActive,
    SessionsACD,
}

static mut REGPTR: *mut Arc<Mutex<Registry>> = 0 as *mut Arc<Mutex<Registry>>;

lazy_static! {
    static ref USER_COUNTERS: Mutex<HashMap<String, Arc<Mutex<Counter>>>> = {
        Mutex::new(HashMap::new())
    };
    static ref USER_GAUGES: Mutex<HashMap<String, Arc<Mutex<Gauge>>>> = {
        Mutex::new(HashMap::new())
    };
    static ref COUNTERS: [Arc<Mutex<Counter>>;14] = {[
        Arc::new(Mutex::new(Counter::new("freeswitch_heartbeats_total".to_string(),
                                         "FreeSWITCH heartbeat count".to_string()))),

        Arc::new(Mutex::new(prometheus::Counter::new("freeswitch_sessions_total".to_string(),
                                                     "FreeSWITCH Session Count".to_string()))),

        Arc::new(Mutex::new(prometheus::Counter::new("freeswitch_sessions_answered_total".to_string(),
                                                     "FreeSWITCH Answered Sessions Count".to_string()))),

        Arc::new(Mutex::new(prometheus::Counter::new("freeswitch_sessions_failed_total".to_string(),
                                                     "FreeSWITCH Failed Sessions Count".to_string()))),

        Arc::new(Mutex::new(prometheus::Counter::new("freeswitch_sessions_inbound_total".to_string(),
                                                     "FreeSWITCH Inbound Sessions Count".to_string()))),

        Arc::new(Mutex::new(prometheus::Counter::new("freeswitch_sessions_inbound_answered_total".to_string(),
                                                     "FreeSWITCH Answered Inbound Sessions Count".to_string()))),

        Arc::new(Mutex::new(prometheus::Counter::new("freeswitch_sessions_inbound_failed_total".to_string(),
                                                     "FreeSWITCH Failed Inbound Sessions Count".to_string()))),

        Arc::new(Mutex::new(prometheus::Counter::new("freeswitch_sessions_outbound_total".to_string(),
                                                     "FreeSWITCH Outbound Sessions Count".to_string()))),

        Arc::new(Mutex::new(prometheus::Counter::new("freeswitch_sessions_outbound_answered_total".to_string(),
                                                     "FreeSWITCH Answered Outbound Sessions Count".to_string()))),

        Arc::new(Mutex::new(prometheus::Counter::new("freeswitch_sessions_outbound_failed_total".to_string(),
                                                     "FreeSWITCH Failed Outbound Sessions Count".to_string()))),

        // Registration Metrics
        Arc::new(Mutex::new(prometheus::Counter::new("freeswitch_registrations_total".to_string(),
                                                     "FreeSWITCH Registration Count".to_string()))),

        Arc::new(Mutex::new(prometheus::Counter::new("freeswitch_registration_attempts_total".to_string(),
                                                     "FreeSWITCH Registration Attempts".to_string()))),

        Arc::new(Mutex::new(prometheus::Counter::new("freeswitch_registration_failures_total".to_string(),
                                                     "FreeSWITCH Registration Failures".to_string()))),
        // calls duration metric
        Arc::new(Mutex::new(prometheus::Counter::new("freeswitch_sessions_outbound_duration_total".to_string(),
                                                     "FreeSWITCH outbound Calls total duration".to_string())))
    ]};
    static ref GAUGES: [Arc<Mutex<Gauge>>;4] = {[
        Arc::new(Mutex::new(prometheus::Gauge::new("freeswitch_sessions_active".to_string(),
                                                   "FreeSWITCH Active Sessions".to_string()))),

        Arc::new(Mutex::new(prometheus::Gauge::new("freeswitch_sessions_asr".to_string(),
                                                   "FreeSWITCH Sessions Answer Seizure Ratio".to_string()))),

        Arc::new(Mutex::new(prometheus::Gauge::new("freeswitch_registrations_active".to_string(),
                                                   "FreeSWITCH Active Registrations".to_string()))),

        Arc::new(Mutex::new(prometheus::Gauge::new("freeswitch_sessions_acd".to_string(),
                                                    "FreeSWITCH outbound Calls Average Duration".to_string()))),
    ]};
    static ref EVENT_NODE_IDS: Mutex<Vec<u64>> = {
        Mutex::new(Vec::new())
    };
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
    unsafe {
        // FIXME: use config api to fetch the port from a config file
        let reg = Box::new(Arc::new(Mutex::new(Registry::new("0.0.0.0".to_string(), 9282))));
        REGPTR = Box::into_raw(reg);
    };
    let reg = unsafe { &*REGPTR };
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
    let mut id = freeswitchrs::event_bind("mod_prometheus", fsr::event_types::HEARTBEAT, None, |_| {
        COUNTERS[FSCounter::Heartbeats].lock().unwrap().increment();
    });
    EVENT_NODE_IDS.lock().unwrap().push(id);

    // New channel created
    id = freeswitchrs::event_bind("mod_prometheus", fsr::event_types::CHANNEL_CREATE, None, |e| {
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
            fslog!(WARNING, "Received channel create event with no call direction: {:?}\n", b);
        }
    });
    EVENT_NODE_IDS.lock().unwrap().push(id);

    // Channel answered
    id = freeswitchrs::event_bind("mod_prometheus", fsr::event_types::CHANNEL_ANSWER, None, |e| {
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
            fslog!(WARNING, "Received channel answer event with no call direction: {:?}\n", b);
        }
    });
    EVENT_NODE_IDS.lock().unwrap().push(id);

    // Channel hangup
    id = freeswitchrs::event_bind("mod_prometheus", fsr::event_types::CHANNEL_HANGUP, None, |e| {
        if let Some(answer) = e.header("Caller-Channel-Answered-Time") {
            let parsed_time = answer.parse::<i64>();
            if parsed_time.is_ok() {
                let myduration = parsed_time.unwrap() ;

                if myduration == 0 {
                    if let Some(direction) = e.header("Call-Direction") {
                        if direction == "inbound" {
                            COUNTERS[FSCounter::SessionsInboundFailed].lock().unwrap().increment();
                        } else {
                            COUNTERS[FSCounter::SessionsOutboundFailed].lock().unwrap().increment();
                        }
                        COUNTERS[FSCounter::SessionsFailed].lock().unwrap().increment();
                    } else {
                        let b = e.body().unwrap_or(Cow::Borrowed("<No Body>"));
                        fslog!(WARNING, "Received channel hangup event with no call direction: {:?}\n", b);
                    }
                }

                if myduration > 0  {
                    if let Some(direction) = e.header("Call-Direction") {
                        if direction != "inbound" {
                            for _i in 1..=myduration  {
                                COUNTERS[FSCounter::SessionsOutboundCallDurationTotal].lock().unwrap().increment();
                            }
                            let answered = COUNTERS[FSCounter::SessionsOutboundAnswered].lock().unwrap().value(); // same used for ASR computation
                            if answered > 0.0 {
                                let acd = COUNTERS[FSCounter::SessionsOutboundCallDurationTotal].lock().unwrap().value() / answered;
                                GAUGES[FSGauge::SessionsACD].lock().unwrap().set(acd);
                            }
                        }
                    }
                }
            }
        } else {
            let b = e.body().unwrap_or(Cow::Borrowed("<No Body>"));
            fslog!(WARNING, "Received channel hangup event with no call answer time information: {:?}\n", b);
        }
    });
    EVENT_NODE_IDS.lock().unwrap().push(id);

    // Channel destroyed
    id = freeswitchrs::event_bind("mod_prometheus", fsr::event_types::CHANNEL_DESTROY, None, |_| {
        GAUGES[FSGauge::SessionsActive].lock().unwrap().decrement();
    });
    EVENT_NODE_IDS.lock().unwrap().push(id);

    // FIXME: Registrations are bound to be outdated on restart (registrations are in the db)
    // so we should fetch them on module load to get the counters initialized

    // Registration attempts
    id = freeswitchrs::event_bind("mod_prometheus", fsr::event_types::CUSTOM, Some("sofia::register_attempt"), |_| {
        COUNTERS[FSCounter::RegistrationAttempts].lock().unwrap().increment();
    });
    EVENT_NODE_IDS.lock().unwrap().push(id);

    // Registration failures
    id = freeswitchrs::event_bind("mod_prometheus", fsr::event_types::CUSTOM, Some("sofia::register_failure"), |_| {
        COUNTERS[FSCounter::RegistrationFailures].lock().unwrap().increment();
    });
    EVENT_NODE_IDS.lock().unwrap().push(id);

    // Registration counters
    id = freeswitchrs::event_bind("mod_prometheus", fsr::event_types::CUSTOM, Some("sofia::register"), |_| {
        COUNTERS[FSCounter::Registrations].lock().unwrap().increment();
        GAUGES[FSGauge::RegistrationsActive].lock().unwrap().increment();
    });
    EVENT_NODE_IDS.lock().unwrap().push(id);

    id = freeswitchrs::event_bind("mod_prometheus", fsr::event_types::CUSTOM, Some("sofia::unregister"), |_| {
        GAUGES[FSGauge::RegistrationsActive].lock().unwrap().decrement();
    });
    EVENT_NODE_IDS.lock().unwrap().push(id);

    id = freeswitchrs::event_bind("mod_prometheus", fsr::event_types::CUSTOM, Some("sofia::expire"), |_| {
        GAUGES[FSGauge::RegistrationsActive].lock().unwrap().decrement();
    });
    EVENT_NODE_IDS.lock().unwrap().push(id);

    /* APIs */
    mod_int.add_raw_api("prom_counter_increment", "Increment Counter", "Increment Counter", counter_increment_api);
    mod_int.add_raw_api("prom_gauge_set", "Set Gauge Value", "Set Gauge Value", gauge_set_api);
    mod_int.add_raw_api("prom_gauge_increment", "Increase Gauge Value", "Increase Gauge Value", gauge_increment_api);
    mod_int.add_raw_api("prom_gauge_decrement", "Decrement Gauge Value", "Decrement Gauge Value", gauge_decrement_api);

    /* Applications */
    mod_int.add_raw_application("prom_gauge_increment",
                                "Increment Gauge", "Increment Gauge",
                                "prom_gauge_increment <gauge> [<value>]",
                                gauge_increment_app,
                                fsr::application_flag_enum::SUPPORT_NOMEDIA);

    fslog!(INFO, "Loaded Prometheus Metrics Module");
    Ok(())
}

fn parse_metric_api_args(cmd: *const std::os::raw::c_char,
                         stream: Option<*mut fsr::stream_handle>)
                         -> Option<(String, f64)> {
    let cmdopt = unsafe { fsr::ptr_to_str(cmd) };
    if !cmdopt.is_some() {
        if let Some(s) = stream {
            unsafe { (*s).write_function.unwrap()(s, fsr::str_to_ptr("Invalid arguments")); }
        } else {
            fslog!(ERROR, "Invalid metric arguments");
        }
        return None;
    }
    let cmdstr = cmdopt.unwrap();
    let args: Vec<&str> = cmdstr.split(' ').collect();
    let name = args[0];
    let val = if args.len() > 1 {
        let r = args[1].parse::<f64>();
        if r.is_ok() {
            r.unwrap()
        } else {
            if let Some(s) = stream {
                unsafe { (*s).write_function.unwrap()(s, fsr::str_to_ptr("Invalid metric value")); }
            } else {
                fslog!(ERROR, "Invalid metric value");
            }
            return None;
        }
    } else { 1 as f64 };
    Some((name.to_string(), val))
}

#[allow(unused_variables)]
unsafe extern "C" fn counter_increment_api(cmd: *const std::os::raw::c_char,
                                           session: *mut fsr::core_session,
                                           stream: *mut fsr::stream_handle)
                                           -> fsr::status {
    let argsopt = parse_metric_api_args(cmd, Some(stream));
    if !argsopt.is_some() {
        return fsr::status::FALSE;
    }
    let v: f64;
    let (name, val) = argsopt.unwrap();
    {
        let mut counters = USER_COUNTERS.lock().unwrap();
        if !counters.contains_key(&name) {
            let counter = Arc::new(Mutex::new(Counter::new(name.clone(), name.clone())));
            counters.insert(name.clone(), counter.clone());
            let reg = &*REGPTR;
            reg.lock().unwrap().register_counter(counter);
        }
        v = counters[&name].lock().unwrap().increment_by(val);
    }
    let out = format!("+OK {}", v);
    (*stream).write_function.unwrap()(stream, fsr::str_to_ptr(&out));
    fsr::status::SUCCESS
}

fn gauge_get(name: &str) -> Arc<Mutex<Gauge>> {
    let mut gauges = USER_GAUGES.lock().unwrap();
    if gauges.contains_key(name) {
        gauges[name].clone()
    } else {
        let gauge = Arc::new(Mutex::new(Gauge::new(name.to_string(), name.to_string())));
        gauges.insert(name.to_string(), gauge.clone());
        let reg = unsafe { &*REGPTR };
        reg.lock().unwrap().register_gauge(gauge.clone());
        gauge
    }
}

#[allow(unused_variables)]
unsafe extern "C" fn gauge_set_api(cmd: *const std::os::raw::c_char,
                                   session: *mut fsr::core_session,
                                   stream: *mut fsr::stream_handle)
                                   -> fsr::status {
    let argsopt = parse_metric_api_args(cmd, Some(stream));
    if !argsopt.is_some() {
        return fsr::status::FALSE;
    }
    let (name, val) = argsopt.unwrap();
    let gauge = gauge_get(&name);
    let v = gauge.lock().unwrap().set(val);
    let out = format!("+OK {}", v);
    (*stream).write_function.unwrap()(stream, fsr::str_to_ptr(&out));
    fsr::status::SUCCESS
}

#[allow(unused_variables)]
unsafe extern "C" fn gauge_increment_api(cmd: *const std::os::raw::c_char,
                                         session: *mut fsr::core_session,
                                         stream: *mut fsr::stream_handle)
                                         -> fsr::status {
    let argsopt = parse_metric_api_args(cmd, Some(stream));
    if !argsopt.is_some() {
        return fsr::status::FALSE;
    }
    let (name, val) = argsopt.unwrap();
    let gauge = gauge_get(&name);
    let v = gauge.lock().unwrap().increment_by(val);
    let out = format!("+OK {}", v);
    (*stream).write_function.unwrap()(stream, fsr::str_to_ptr(&out));
    fsr::status::SUCCESS
}

#[allow(unused_variables)]
unsafe extern "C" fn gauge_decrement_api(cmd: *const std::os::raw::c_char,
                                         session: *mut fsr::core_session,
                                         stream: *mut fsr::stream_handle)
                                         -> fsr::status {
    let argsopt = parse_metric_api_args(cmd, Some(stream));
    if !argsopt.is_some() {
        return fsr::status::FALSE;
    }
    let (name, val) = argsopt.unwrap();
    let gauge = gauge_get(&name);
    let v = gauge.lock().unwrap().decrement_by(val);
    let out = format!("+OK {}", v);
    (*stream).write_function.unwrap()(stream, fsr::str_to_ptr(&out));
    fsr::status::SUCCESS
}

#[allow(unused_variables)]
unsafe extern "C" fn gauge_increment_app(session: *mut fsr::core_session,
                                         data: *const std::os::raw::c_char) {
    let argsopt = parse_metric_api_args(data, None);
    if argsopt.is_some() {
        let (name, val) = argsopt.unwrap();
        let gauge = gauge_get(&name);
        let v = gauge.lock().unwrap().increment_by(val);
        fslog!(INFO, "Incremented gauge {} to {}", name, v);
    }
}

fn prometheus_unload() -> Status {
    let reg = unsafe { &*REGPTR };
    USER_GAUGES.lock().unwrap().clear();
    USER_COUNTERS.lock().unwrap().clear();
    {
        let mut event_ids = EVENT_NODE_IDS.lock().unwrap();
        for e in event_ids.iter() {
            freeswitchrs::event_unbind(*e);
        }
        event_ids.clear();
    }
    fslog!(DEBUG, "Stopping metric registry");
    Registry::stop(&reg);
    std::mem::drop(reg);
    unsafe {
        REGPTR = 0 as *mut Arc<Mutex<Registry>>;
    }
    fslog!(DEBUG, "Metric registry destroyed");
    Ok(())
}

static MOD_PROMETHEUS_DEF: ModDefinition = ModDefinition {
    name: "mod_prometheus",
    load: prometheus_load,
    runtime: None,
    shutdown: Some(prometheus_unload)
};

freeswitch_export_mod!(mod_prometheus_module_interface, MOD_PROMETHEUS_DEF);
