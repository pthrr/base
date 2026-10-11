m4_dnl One periodic consumer input port with OneItem and DropOldest.
m4_dnl Queue size N = 2. The source sends at most K = 2 times in any window
m4_dnl from the start of one dispatch to the end of the next, so the ring has
m4_dnl capacity N + K - 1 = 3. Defining AADL_SPEC swaps the ring for the AADL
m4_dnl DropOldest queue; the two expansions must be weak-trace equivalent.
sort Sample = struct sample(value: Bool);

RTCHAN_EVENT_DATA_INPUT(port, Sample, 2, 3)

act port_source_begin, port_source_end, port_begin, port_end;

m4_dnl x counts sends in the oldest open window; y counts sends in the window
m4_dnl opened by the current dispatch.
proc Source(x: Nat, y: Nat, two: Bool) =
    (sum v:Bool. (x < 2 && (!two || y < 2)) ->
      (port_caller_send_ok(sample(v)) + port_caller_send_full(sample(v)))
      . Source(x + 1, if(two, y + 1, 0), two))
  + ((!two) -> port_source_begin . Source(x, 0, true))
  + (two -> port_source_end . Source(y, 0, false));

m4_ifdef(`AADL_SPEC', `
m4_dnl Reuses the port queue action names; the implementation process is unused.
proc AadlQueue(q: List(Sample)) =
    (sum m:Sample. (#q < 2) -> port_queue_send_ok(m) . AadlQueue(q <| m))
  + (sum m:Sample. (#q >= 2) -> port_queue_send_ok(m) . AadlQueue(tail(q) <| m))
  + ((q != []) -> port_queue_recv_ok(head(q)) . AadlQueue(tail(q)))
  + ((q == []) -> port_queue_recv_empty . AadlQueue(q))
  + port_queue_len(#q) . AadlQueue(q);
m4_define(`PORT', `AadlQueue([]) || RTCHAN_EVENT_DATA_INPUT_ADAPTER_INIT(port)')
', `m4_define(`PORT', `RTCHAN_EVENT_DATA_INPUT_INIT(port)')')

init hide({port_len, port_recv_ok, port_recv_empty},
  allow({RTCHAN_EVENT_DATA_INPUT_VISIBLE(port), port_begin, port_end,
         port_complete, port_complete_empty},
    comm({RTCHAN_EVENT_DATA_INPUT_COMM(port),
          port_input_begin|port_source_begin -> port_begin,
          port_input_end|port_source_end -> port_end},
      Source(0, 0, false) || PORT)));
