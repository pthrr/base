m4_dnl One periodic consumer data input port on a sampled connection.
m4_dnl The source sends at most K = 2 times in any window from the start of one
m4_dnl dispatch to the end of the next, so the implementation queue has capacity K = 2.
m4_dnl Defining AADL_SPEC replaces the implementation port with a
m4_dnl register that holds the latest value; the two expansions must be
m4_dnl weak-trace equivalent.
sort Sample = struct sample(value: Bool);

RTCHAN_DATA_INPUT(port, Sample, 2)

act port_source_begin, port_source_end, port_begin, port_end;
    port_snap_q, port_snap_r, port_snap: port_Latest # Bool;

m4_dnl x counts sends in the oldest open window; y counts sends in the window
m4_dnl opened by the current dispatch.
proc Source(x: Nat, y: Nat, two: Bool) =
    (sum v:Bool. (x < 2 && (!two || y < 2)) ->
      (port_caller_send_ok(sample(v)) + port_caller_send_full(sample(v)))
      . Source(x + 1, if(two, y + 1, 0), two))
  + ((!two) -> port_source_begin . Source(x, 0, true))
  + (two -> port_source_end . Source(y, 0, false));

m4_ifdef(`AADL_SPEC', `
m4_dnl Reuses the declared action names; the rtchan processes are unused.
proc Register(latest: port_Latest, fresh: Bool) =
    (sum m:Sample. port_queue_send_ok(m) . Register(port_some(m), true))
  + port_snap_q(latest, fresh) . Register(latest, false);

proc Reader =
    port_input_begin .
    (sum x:port_Latest, u:Bool. port_snap_r(x, u) .
      port_input_end . port_frozen(x, u) . Reader);

m4_define(`PORT', `Register(port_none, false) || Reader')
m4_define(`PORT_COMM', `port_snap_q|port_snap_r -> port_snap')
', `
m4_define(`PORT', `RTCHAN_DATA_INPUT_INIT(port)')
')

init hide({port_len, port_recv_ok, port_recv_empty, port_snap},
  allow({RTCHAN_DATA_INPUT_VISIBLE(port),
         port_snap, port_begin, port_end},
    comm({RTCHAN_DATA_INPUT_COMM(port),
m4_ifdef(`AADL_SPEC', `          port_snap_q|port_snap_r -> port_snap,')
          port_input_begin|port_source_begin -> port_begin,
          port_input_end|port_source_end -> port_end},
      Source(0, 0, false) || PORT)));
