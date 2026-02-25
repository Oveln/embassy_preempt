/* Custom defmt.x for RISC-V to avoid PCREL_HI20 relocation errors */

EXTERN(_defmt_acquire);
EXTERN(_defmt_release);
EXTERN(__defmt_default_timestamp);
EXTERN(__DEFMT_MARKER_TIMESTAMP_WAS_DEFINED);
PROVIDE(_defmt_timestamp = __defmt_default_timestamp);
PROVIDE(_defmt_panic = __defmt_default_panic);

/* Place defmt section at the beginning of RAM to ensure it's within PC-relative range */
SECTIONS
{
  .defmt 0x41000000 (INFO) :
  {
    . = 0x41000000;

    /* Format implementations for primitives like u8 */
    *(.defmt.prim.*);

    /* Log messages ordered by severity */
    __DEFMT_MARKER_TRACE_START = .;
    *(.defmt.trace.*);
    __DEFMT_MARKER_TRACE_END = .;
    __DEFMT_MARKER_DEBUG_START = .;
    *(.defmt.debug.*);
    __DEFMT_MARKER_DEBUG_END = .;
    __DEFMT_MARKER_INFO_START = .;
    *(.defmt.info.*);
    __DEFMT_MARKER_INFO_END = .;
    __DEFMT_MARKER_WARN_START = .;
    *(.defmt.warn.*);
    __DEFMT_MARKER_WARN_END = .;
    __DEFMT_MARKER_ERROR_START = .;
    *(.defmt.error.*);
    __DEFMT_MARKER_ERROR_END = .;

    /* Everything user-defined */
    *(.defmt.*);

    __DEFMT_MARKER_END = .;

    /* Symbols that aren't referenced by the program */
    KEEP(*(.defmt.end .defmt.end.*));
  } > RAM
}

ASSERT(__DEFMT_MARKER_END - 0x41000000 < 65536, ".defmt section cannot exceed 64KB");