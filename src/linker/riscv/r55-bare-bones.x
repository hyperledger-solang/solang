MEMORY
{
  CALL_DATA : ORIGIN = 0x80000000, LENGTH = 1M
  STACK     : ORIGIN = 0x80100000, LENGTH = 2M
  REST_OF_RAM : ORIGIN = 0x80300000, LENGTH = 1021M
}

SECTIONS
{
  . = 0x80300000;
  .text : { *(.text) } > REST_OF_RAM
  .data : {
    *(.data)
    PROVIDE( __global_pointer$ = . + 0x800 );
  } > REST_OF_RAM
  .bss : { *(.bss) } > REST_OF_RAM

  _stack_top = ORIGIN(STACK) + LENGTH(STACK);
}

ENTRY(_start)
