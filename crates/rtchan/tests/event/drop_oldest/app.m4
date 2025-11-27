m4_dnl Periodic OneItem event input with DropOldest, N = K = 2.
m4_dnl The ring capacity is N + K - 1 = 3. The source bounds arrivals
m4_dnl between dispatch windows. AADL_SPEC replaces the ring with a direct
m4_dnl two-entry DropOldest queue for weak-trace comparison.
RTCHAN_EVENT_INPUT(port, 2)
RTCHAN_CHANNEL(port, port_Occurrence, 3)

act port_source_begin, port_source_end, port_begin, port_end;

m4_dnl oldest_count and current_count count sends in the two open windows.
proc Source(oldest_count: Nat, current_count: Nat, two: Bool) =
    ((oldest_count < 2 && (!two || current_count < 2)) ->
      (port_caller_send_ok(port_tick) + port_caller_send_full(port_tick))
      . Source(oldest_count + 1, if(two, current_count + 1, 0), two))
  + ((!two) -> port_source_begin . Source(oldest_count, 0, true))
  + (two -> port_source_end . Source(current_count, 0, false));

m4_ifdef(`AADL_SPEC', `
proc AadlEventQueue(q: List(port_Occurrence)) =
    (sum occurrence:port_Occurrence. (#q < 2) -> port_queue_send_ok(occurrence) . AadlEventQueue(q <| occurrence))
  + (sum occurrence:port_Occurrence. (#q >= 2) -> port_queue_send_ok(occurrence) . AadlEventQueue(tail(q) <| occurrence))
  + ((q != []) -> port_queue_recv_ok(head(q)) . AadlEventQueue(tail(q)))
  + ((q == []) -> port_queue_recv_empty . AadlEventQueue(q))
  + port_queue_len(#q) . AadlEventQueue(q);
m4_define(`QUEUE', `AadlEventQueue([])')
', `m4_define(`QUEUE', `RTCHAN_CHANNEL_INIT(port)')')

init hide({port_len, port_recv_ok, port_recv_empty},
  allow({RTCHAN_CHANNEL_VISIBLE(port), port_begin, port_end,
         port_complete, port_complete_empty},
    comm({RTCHAN_CHANNEL_COMM(port),
          port_input_begin|port_source_begin -> port_begin,
          port_input_end|port_source_end -> port_end},
      Source(0, 0, false) || RTCHAN_EVENT_INPUT_INIT(port) || QUEUE)));
