FreeSWITCH Prometheus Module
----------------------------

This module exposes FreeSWITCH metrics for scraping by
`Prometheus
<https://prometheus.io/>`_.

mod_prometheus is built upon FreeSWITCH Rust bindings
`freeswitchrs
<https://gitlab.com/wiresight/freeswitchrs/>`_.

You can also use FreeSWITCH ESL APIs to increase custom counters or gauges::

    fscli> prom_counter_increment my_counter

    fscli> prom_counter_increment my_counter 100

    fscli> prom_gauge_set my_gauge 500

    fscli> prom_gauge_increment my_gauge
    fscli> prom_gauge_decrement my_gauge 2

As all FreeSWITCH APIs, these functions can be used from the XML dialplan or the command line.
s can be used from the XML dialplan or the command line.


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

