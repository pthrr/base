m4_dnl Standalone capacity-two FIFO with a caller that may try every operation.
sort Item = struct item_a | item_b;

RTCHAN_CHANNEL(link, Item, 2)

proc Caller =
    (sum message:Item.
      (link_caller_send_ok(message) + link_caller_send_full(message)
       + link_caller_recv_ok(message))) . Caller
  + link_caller_recv_empty . Caller
  + (link_caller_len(0) + link_caller_len(1) + link_caller_len(2)) . Caller;

init allow({RTCHAN_CHANNEL_VISIBLE(link)},
  comm({RTCHAN_CHANNEL_COMM(link)},
    Caller || RTCHAN_CHANNEL_INIT(link)));
