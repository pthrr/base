m4_dnl One sporadic consumer input port with OneItem and DropOldest.
m4_dnl Queue size N = 2, at most K = 2 sends per window, ring capacity
m4_dnl N + K - 1 = 3. The source sends four distinct events in order and
m4_dnl wakes the consumer after each accepted send; the scheduler waits, then
m4_dnl dispatches while input is pending, as in ../sporadic/app.m4. Four events
m4_dnl are enough for a dispatch to find three queued and trim one. Defining
m4_dnl AADL_SPEC swaps the ring for the AADL DropOldest queue; the two
m4_dnl expansions must be weak-trace equivalent.
sort Item = struct i0 | i1 | i2 | i3;

RTCHAN_CHANNEL(port, Item, 3)
RTCHAN_EVENT_DATA_INPUT(port, Item, 2)

act app_caller_notify, app_latch_notify, app_notify,
    app_caller_wait, app_latch_wait, app_wait,
    app_sched_begin, app_sched_end, port_source_begin, port_source_end,
    port_begin, port_end;

proc Latch(set: Bool) =
    app_latch_notify . Latch(true)
  + (set -> app_latch_wait . Latch(false));

m4_dnl rest holds the events still to send; x and y count sends per window as
m4_dnl in ../drop_oldest/app.m4. Each accepted send is followed by a wakeup.
proc Source(rest: List(Item), x: Nat, y: Nat, two: Bool) =
    ((rest != [] && x < 2 && (!two || y < 2)) ->
      (port_caller_send_ok(head(rest)) . app_caller_notify
        + port_caller_send_full(head(rest)))
      . Source(tail(rest), x + 1, if(two, y + 1, 0), two))
  + ((!two) -> port_source_begin . Source(rest, x, 0, true))
  + (two -> port_source_end . Source(rest, y, 0, false));

m4_dnl has_pending() is one length read.
proc Scheduler = app_caller_wait . Drain;
proc Drain = sum n:Nat. port_caller_len(n) .
    ((n > 0) -> app_sched_begin . app_sched_end . Drain <> Scheduler);

m4_ifdef(`AADL_SPEC', `
m4_dnl Reuses the port queue action names; the implementation process is unused.
proc AadlQueue(q: List(Item)) =
    (sum m:Item. (#q < 2) -> port_queue_send_ok(m) . AadlQueue(q <| m))
  + (sum m:Item. (#q >= 2) -> port_queue_send_ok(m) . AadlQueue(tail(q) <| m))
  + ((q != []) -> port_queue_recv_ok(head(q)) . AadlQueue(tail(q)))
  + ((q == []) -> port_queue_recv_empty . AadlQueue(q))
  + port_queue_len(#q) . AadlQueue(q);
m4_define(`QUEUE', `AadlQueue([])')
', `m4_define(`QUEUE', `RTCHAN_CHANNEL_INIT(port)')')

init hide({port_len, port_recv_ok, port_recv_empty},
  allow({RTCHAN_CHANNEL_VISIBLE(port), app_notify, app_wait,
         port_begin, port_end, port_complete, port_complete_empty},
    comm({RTCHAN_CHANNEL_COMM(port),
          app_caller_notify|app_latch_notify -> app_notify,
          app_caller_wait|app_latch_wait -> app_wait,
          port_input_begin|app_sched_begin|port_source_begin -> port_begin,
          port_input_end|app_sched_end|port_source_end -> port_end},
      Source([i0, i1, i2, i3], 0, 0, false) || Scheduler || RTCHAN_EVENT_DATA_INPUT_INIT(port)
        || Latch(false) || QUEUE)));
