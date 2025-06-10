.section .text.entry
.global _start

_start:
    /* Temel sistem ayarları (Örneğin, bellek yönetimi, aygıtların başlatılması vb.) */
    /* Bu kısım sisteme özel olacaktır. */

    /* Yığını (stack) ayarla */
    /* Yığın adresi ve boyutu sisteme göre belirlenir. */
    lis r1, _stack_top@h  /* Yığının en üst adresinin yüksek yarısını yükle */
    ori r1, r1, _stack_top@l /* Yığının en üst adresinin düşük yarısını yükle */

    /* Global pointer'ı (gp) ayarla (Gerekirse) */
    /* Bazı PowerPC sistemlerinde global verilere erişim için gp register'ı kullanılır. */
    /* Eğer kullanılıyorsa, uygun şekilde ayarlanmalıdır. */

    /* Ana fonksiyona atla */
    bl main /* main fonksiyonuna dallan ve bağlantıyı kaydet */

    /* Sonsuz döngü (main fonksiyonuReturn etmezse) */
    halt:
    b halt

.section .data
/* Yığın için ayrılan alan */
.align 8 /* 8 byte hizalama */
_stack_bottom:
.space 16384 /* 16KB yığın */
_stack_top:

.end
