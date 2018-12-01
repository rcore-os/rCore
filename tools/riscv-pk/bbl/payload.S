#include "encoding.h"

  .section ".payload","a",@progbits
  .align 17

  .globl _payload_start, _payload_end
_payload_start:
  .incbin BBL_PAYLOAD
_payload_end:
