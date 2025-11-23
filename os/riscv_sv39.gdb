define riscv_sv39_lookup
    if $argc != 2
        printf "usage: riscv_sv39_looup(VA, satp)\n"
        return
    end

    set $va = $arg0
    set $satp_pa = $arg1
    set $va_39 = $va & ((1<<39)-1)
    set $vpn2 = ($va_39 >> 31) & 0x1FF
    set $vpn1 = ($va_39 >> 22) & 0x1FF
    set $vpn0 = ($va_39 >> 13) & 0x1FF

    set $root_pa = $satp_pa << 12
    set $pte1_pa = $root_pa + $vpn2 * 8
    set $pte1 = *(uint64_t*)($pte1_pa)
    set $pte1_ppn = $pte1 >> 10
    set $l2_table_pa = $pte1_ppn << 12
    set $pte2_pa = $l2_table_pa + $vpn1 * 8
    set $pte2 = *(uint64_t*)($pte2_pa)
    set $pte2_ppn = $pte2 >> 10
    set $l3_table_pa = $pte2_ppn << 12
    set $pte3_pa = $l3_table_pa + $vpn0 * 8
    set $pte3 = *(uint64_t*)($pte3_pa)

    printf "vpn indexes: %d, %d, %d\n", $vpn2, $vpn1, $vpn0
    printf "l1_pte: addr=0x%x, ppn=0x%x, privilege=b%t\n", $pte1, $pte1_ppn, $pte1 & ((1<<10)-1)
    printf "l2_pte: addr=0x%x, ppn=0x%x, privilege=b%t\n", $pte2, $pte2_ppn, $pte2 & ((1<<10)-1)
    printf "l3_pte: addr=0x%x, ppn=0x%x, privilege=b%t\n", $pte1, $pte3_ppn, $pte3 & ((1<<10)-1)
end

define show_priv
    if $argc != 1
        printf "only one arg\n"
        return
    end

    set $p = $arg0 & ((1<<12)-1)
    p/t $p
    return
end

define get_pte
    if $argc != 1
        printf "only one arg\n"
        return
    end
    p/x ($arg0 >> 10 << 12)
end
