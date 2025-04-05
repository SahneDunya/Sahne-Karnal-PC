.section .text.entry
.globl _start

_start:
    # mhartid'yi a0'a yükle (hangi çekirdekte çalıştığımızı belirlemek için)
    mv a0, tp

    # Global pointer'ı ayarla (gp)
    la gp, __global_pointer$

    # Stack pointer'ı ayarla (sp)
    la sp, _stack_top

    # mstatus register'ını ayarla (kesmeleri devre dışı bırak, supervisor moduna geç)
    # Daha güvenli ve okunabilir bir yaklaşım:
    csrci mstatus, MSTATUS_MIE # Global kesmeleri devre dışı bırak
    li t0, MSTATUS_SPP | MSTATUS_MPP # Supervisor modunu ayarla
    csrs mstatus, t0
    
    # medeleg ve mideleg register'larını ayarla (istisna ve kesme delegasyonu)
    # Tüm bitleri ayarlamak yerine, gerekli olanları ayarlamak daha iyi.
    # Örneğin, supervisor seviyesindeki kesmeleri delege etmek için:
    li t0, 0xB000000000000000 # Supervisor seviyesindeki kesmeler için delegasyon bitleri (örnek)
    csrw medeleg, t0
    csrw mideleg, t0
    # VEYA
    # Supervisor seviyesindeki tüm kesmeleri delege etmek için:
    li t0, 0xffffffff
    csrw medeleg, t0
    csrw mideleg, t0
    
    # mepc register'ına kernel_main adresini yükle
    la t0, kernel_main
    csrw mepc, t0

    # mhartid'yi a0'a geri yükle (kernel_main'e argüman olarak geçirmek için)
    mv a0, tp

    # mret komutu ile supervisor moduna geç ve kernel_main'i çalıştır
    mret

.section .bss
.align 8
_stack_top:
    .space 16384 # 16KB stack

    # Daha iyi bir yaklaşım: stack'in boyutunu bir sembol ile tanımlamak
    .equ STACK_SIZE, 16384
    .space STACK_SIZE
_stack_top: