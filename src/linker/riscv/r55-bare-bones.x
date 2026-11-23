/* r55 only maps memory described by PT_LOAD segments, so every section must
   be placed in one. */

MEMORY
{
  CALL_DATA : ORIGIN = 0x80000000, LENGTH = 1M
  STACK     : ORIGIN = 0x80100000, LENGTH = 2M
  REST_OF_RAM : ORIGIN = 0x80300000, LENGTH = 1021M
}

SECTIONS
{
  . = 0x80300000;

  .text : {
    *(.text.start)
    *(.text .text.*)
  } > REST_OF_RAM

  .rodata : {
    *(.rodata .rodata.*)
    *(.srodata .srodata.*)
  } > REST_OF_RAM

  .data : {
    *(.data .data.*)
    . = ALIGN(8);
    PROVIDE( __global_pointer$ = . + 0x800 );
    *(.sdata .sdata.*)
  } > REST_OF_RAM

  .bss (NOLOAD) : {
    *(.sbss .sbss.*)
    *(.bss .bss.*)
    *(COMMON)
  } > REST_OF_RAM

  _stack_top = ORIGIN(STACK) + LENGTH(STACK);

  /DISCARD/ : {
    *(.eh_frame*)
    *(.comment)
    *(.riscv.attributes)
  }
}

ENTRY(_start)
