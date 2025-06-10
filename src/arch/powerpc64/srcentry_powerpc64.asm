.global _start

_start:
    /* 1. Çekirdek moduna geçiş (eğer gerekiyorsa) */
    /* Bu adım, bazı PowerPC sistemlerinde gerekebilir. */
    /* Örneğin, MMU ayarlarını yapmak veya supervisor moduna geçmek gibi. */
    /* Detaylar için donanım belgelerine bakın. */

    /* 2. Yığın (stack) ayarları */
    /* Yığın için bir alan ayırın ve yığın işaretçisini (SP) ayarlayın. */
    lis     r1, _stack_top@h
    ori     r1, r1, _stack_top@l
    mtspr   SPRG0, r1  /* SPRG0 genellikle yığın için kullanılır */
    addi    r1, r1, -STACK_SIZE  /* Yığın boyutu kadar aşağıya in */
    mtsp    r1

    /* 3. BSS bölümünü temizleme */
    /* BSS bölümü, sıfır değerlerle başlatılması gereken verileri içerir. */
    lis     r1, _bss_start@h
    ori     r1, r1, _bss_start@l
    lis     r2, _bss_end@h
    ori     r2, r2, _bss_end@l

clear_bss_loop:
    stw     r0, r1
    addi    r1, r1, 4
    cmpw    r1, r2
    blt     clear_bss_loop

    /* 4. Çekirdek fonksiyonunu çağırma */
    /* Çekirdek kodunuzun bulunduğu fonksiyonu çağırın. */
    /* Örneğin, "kernel_main" veya benzeri bir fonksiyon. */
    bl      kernel_main

    /* 5. (İsteğe bağlı) Sonsuz döngü */
    /* Çekirdek fonksiyonu geri döndüğünde, sistemin çökmesini önlemek için sonsuz bir döngüye girebilirsiniz. */
halt_loop:
    b       halt_loop

.section .bss
.align 4
_stack_top:
    .space STACK_SIZE
_bss_start:
    .space 0
_bss_end:

.section .text
.global kernel_main
kernel_main:
    /* Çekirdek kodunuz buraya gelecek */
    /* ... */

STACK_SIZE = 0x2000 /* Örnek yığın boyutu (8KB) */
