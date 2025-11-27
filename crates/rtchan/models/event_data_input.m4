m4_dnl OneItem dispatch adapter for an AADL event data input port.
m4_dnl Compose with RTCHAN_CHANNEL using the same prefix and payload sort.
m4_dnl $3 is the AADL queue size; channel capacity is chosen separately.

m4_define(`RTCHAN_EVENT_DATA_INPUT', `
act $1_input_begin, $1_input_end;
    $1_complete: $2;
    $1_complete_empty;

proc $1_EventDataInput =
    $1_input_begin .
    (sum queued:Nat. $1_caller_len(queued) .
      $1_EventDataTrim(if(queued > $3, Int2Nat(queued - $3), 0)));

proc $1_EventDataTrim(excess: Nat) =
    (excess > 0)
      -> (sum message:$2. $1_caller_recv_ok(message) . $1_EventDataTrim(Int2Nat(excess - 1)))
      <> $1_EventDataFreeze;

proc $1_EventDataFreeze =
    (sum message:$2. $1_caller_recv_ok(message) . $1_input_end . $1_complete(message) . $1_EventDataInput)
  + $1_caller_recv_empty . $1_input_end . $1_complete_empty . $1_EventDataInput;
')

m4_define(`RTCHAN_EVENT_DATA_INPUT_INIT', `$1_EventDataInput')
