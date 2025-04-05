MEMORY
{
  rom (rx!wa) : ORIGIN = 0x00000000, LENGTH = 1M   /* ROM başlangıç adresi ve boyutu */
  ram (rwx!a) : ORIGIN = 0x80000000, LENGTH = 128M /* RAM başlangıç adresi ve boyutu */
  flash (rx!wa): ORIGIN = 0x20000000, LENGTH = 4M  /* Flash bellek */
}