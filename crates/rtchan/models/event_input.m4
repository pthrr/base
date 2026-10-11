m4_dnl OneItem AADL event input port with a singleton occurrence sort.
m4_dnl $2 is the AADL queue size; $3 is the physical channel capacity.
m4_define(`RTCHAN_EVENT_INPUT', `
sort $1_Occurrence = struct $1_tick;
RTCHAN_CHANNEL($1, $1_Occurrence, $3)
act $1_input_begin, $1_input_end;
    $1_complete: $1_Occurrence;
    $1_complete_empty;

proc $1_EventInput =
    $1_input_begin .
    (sum queued:Nat. $1_caller_len(queued) .
      ((queued == 0)
        -> $1_input_end . $1_complete_empty . $1_EventInput
        <> $1_EventTrim(if(queued > $2, Int2Nat(queued - $2), 0))));

proc $1_EventTrim(excess: Nat) =
    (excess > 0)
      -> $1_caller_recv_ok($1_tick) . $1_EventTrim(Int2Nat(excess - 1))
      <> $1_EventFreeze;

proc $1_EventFreeze =
    $1_caller_recv_ok($1_tick) . $1_input_end .
      $1_complete($1_tick) . $1_EventInput;
')

m4_define(`RTCHAN_EVENT_INPUT_ADAPTER_INIT', `$1_EventInput')
m4_define(`RTCHAN_EVENT_INPUT_INIT', `RTCHAN_CHANNEL_INIT($1) || $1_EventInput')
m4_define(`RTCHAN_EVENT_INPUT_COMM', `RTCHAN_CHANNEL_COMM($1)')
m4_define(`RTCHAN_EVENT_INPUT_VISIBLE', `RTCHAN_CHANNEL_VISIBLE($1)')
