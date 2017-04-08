FreeSWITCH Prometheus Module
----------------------------

This module exposes FreeSWITCH metrics for scraping by
`Prometheus
<https://prometheus.io/>`_.

mod_prometheus is built upon  
`FreeSWITCH Rust bindings
<https://gitlab.com/wiresight/freeswitchrs/>`_.


Install
=======

Installation instructions::

    # Check for and install the Rust compiler
    $ rustc -V || curl -sSf https://static.rust-lang.org/rustup.sh | sh

    # Clone the project and build it:
    $ git clone https://github.com/moises-silva/mod_prometheus.git
    $ cd mod_prometheus
    $ cargo build

    # Copy the module to your FreeSWITCH modules directory:
    $ sudo cp target/debug/libmod_prometheus.so `fs_cli -x 'global_getvar mod_dir'`

    # Load the module:
    $ fs_cli -x 'load libmod_prometheus'

    # Make sure it's loaded and listening to TCP port 6780
    $ fs_cli -x 'module_exists libmod_prometheus'
    true

    $ netstat -nl | grep 6780
    tcp        0      0 0.0.0.0:6780            0.0.0.0:*               LISTEN
    
    # For auto-load the module add this line at the end of your modules.conf 
    $ sudo vi /etc/freeswitch/autoload_configs/modules.conf.xml

        <load module="libmod_prometheus"/>
    
Now you can access your host at port 6780 to check your metrics:
http://YOUR_HOST:6780/


Metrics
=======

These are the metrics provided by default::

    freeswitch_heartbeats
    freeswitch_registration_attempts
    freeswitch_registration_failures
    freeswitch_registrations
    freeswitch_registrations_active
    freeswitch_sessions
    freeswitch_sessions_active
    freeswitch_sessions_answered
    freeswitch_sessions_asr
    freeswitch_sessions_failed
    freeswitch_sessions_inbound
    freeswitch_sessions_inbound_answered
    freeswitch_sessions_inbound_failed
    freeswitch_sessions_outbound
    freeswitch_sessions_outbound_answered
    freeswitch_sessions_outbound_failed

You can also use FreeSWITCH ESL APIs to create your own counters or gauges::

    fscli> prom_counter_increment my_counter

    fscli> prom_counter_increment my_counter 100

    fscli> prom_gauge_set my_gauge 500

    fscli> prom_gauge_increment my_gauge
    fscli> prom_gauge_decrement my_gauge 2

As all FreeSWITCH APIs, these functions can be used from the XML dialplan or the command line.
