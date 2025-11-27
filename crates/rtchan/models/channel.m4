m4_dnl Bounded SPSC FIFO used by the input adapters or directly by a user model.
m4_dnl $1 is the instance prefix, $2 the payload sort, and $3 the positive capacity.
m4_changecom()m4_dnl Preserve mCRL2's #list length operator.

m4_define(`RTCHAN_CHANNEL', `
act $1_queue_send_ok, $1_caller_send_ok, $1_send_ok,
    $1_queue_send_full, $1_caller_send_full, $1_send_full,
    $1_queue_recv_ok, $1_caller_recv_ok, $1_recv_ok: $2;
    $1_queue_recv_empty, $1_caller_recv_empty, $1_recv_empty;
    $1_queue_len, $1_caller_len, $1_len: Nat;

proc $1_Channel(queue: List($2)) =
    (sum message:$2. (#queue < $3) -> $1_queue_send_ok(message) . $1_Channel(queue <| message))
  + (sum message:$2. (#queue >= $3) -> $1_queue_send_full(message) . $1_Channel(queue))
  + ((queue != []) -> $1_queue_recv_ok(head(queue)) . $1_Channel(tail(queue)))
  + ((queue == []) -> $1_queue_recv_empty . $1_Channel(queue))
  + $1_queue_len(#queue) . $1_Channel(queue);
')

m4_define(`RTCHAN_CHANNEL_COMM', `$1_queue_send_ok|$1_caller_send_ok -> $1_send_ok,
    $1_queue_send_full|$1_caller_send_full -> $1_send_full,
    $1_queue_recv_ok|$1_caller_recv_ok -> $1_recv_ok,
    $1_queue_recv_empty|$1_caller_recv_empty -> $1_recv_empty,
    $1_queue_len|$1_caller_len -> $1_len')

m4_define(`RTCHAN_CHANNEL_VISIBLE', `$1_send_ok, $1_send_full,
    $1_recv_ok, $1_recv_empty, $1_len')

m4_define(`RTCHAN_CHANNEL_INIT', `$1_Channel([])')
