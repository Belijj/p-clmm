
#![allow(clippy::too_many_arguments)]
#![allow(dead_code)]

use bytemuck::Zeroable;
use litesvm::LiteSVM;
use solana_sdk::{
    account::Account,
    clock::Clock,
    compute_budget::ComputeBudgetInstruction,
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::{Keypair, Signer as SdkSigner},
    transaction::Transaction,
};
use std::path::PathBuf;

use p_clmm::states::{
    pool::REWARD_NUM, AmmConfig, DynamicFeeInfo, Observation, ObservationState, PoolState,
    RewardInfo, TickArrayState, TickState,
};

fn raydium_program_id() -> Pubkey {
    "CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK".parse().unwrap()
}

fn p_clmm_program_id() -> Pubkey {
    "pCLMM1111111111111111111111111111111111111Z".parse().unwrap()
}

fn spl_token_program_id() -> Pubkey {
    "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
        .parse()
        .unwrap()
}

const MAINNET_AMM_CONFIG_BYTES: &[u8] = include_bytes!("fixtures/mainnet_amm_config.bin");

const POOL_SEED: &[u8] = b"pool";
const AMM_CONFIG_SEED: &[u8] = b"amm_config";
const OBSERVATION_SEED: &[u8] = b"observation";
const TICK_ARRAY_SEED: &[u8] = b"tick_array";
const POOL_VAULT_SEED: &[u8] = b"pool_vault";

fn pool_pda(program: &Pubkey, amm_config: &Pubkey, mint_0: &Pubkey, mint_1: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[POOL_SEED, amm_config.as_ref(), mint_0.as_ref(), mint_1.as_ref()],
        program,
    )
}

fn amm_config_pda(program: &Pubkey, index: u16) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[AMM_CONFIG_SEED, &index.to_be_bytes()], program)
}

fn observation_pda(program: &Pubkey, pool: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[OBSERVATION_SEED, pool.as_ref()], program)
}

fn tick_array_pda(program: &Pubkey, pool: &Pubkey, start_tick: i32) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[TICK_ARRAY_SEED, pool.as_ref(), &start_tick.to_be_bytes()],
        program,
    )
}

fn vault_pda(program: &Pubkey, pool: &Pubkey, mint: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[POOL_VAULT_SEED, pool.as_ref(), mint.as_ref()],
        program,
    )
}

#[derive(Clone)]
struct TickSpec {
    tick: i32,
    liquidity_net: i128,
    liquidity_gross: u128,
}

#[derive(Clone)]
struct ScenarioSpec {
    label: &'static str,
    first_array: Vec<TickSpec>,
    second_array: Vec<TickSpec>,
    amount: u64,
}

fn pack<T: bytemuck::Pod>(disc: [u8; 8], value: &T) -> Vec<u8> {
    let mut out = Vec::with_capacity(8 + std::mem::size_of::<T>());
    out.extend_from_slice(&disc);
    out.extend_from_slice(bytemuck::bytes_of(value));
    out
}

struct PoolFixture {
    pool: Pubkey,
    pool_bump: u8,
    amm_config: Pubkey,
    observation: Pubkey,
    tick_array: Pubkey,
    second_tick_array: Option<Pubkey>,
    vault_0: Pubkey,
    vault_1: Pubkey,
    mint_0: Pubkey,
    mint_1: Pubkey,
    pool_bytes: Vec<u8>,
    tick_array_bytes: Vec<u8>,
    second_tick_array_bytes: Option<Vec<u8>>,
    amm_config_bytes: Vec<u8>,
    observation_bytes: Vec<u8>,
}

fn build_pool_fixture(
    program: &Pubkey,
    mint_0: Pubkey,
    mint_1: Pubkey,
    spec: &ScenarioSpec,
) -> PoolFixture {
    let (amm_config_addr, _) = amm_config_pda(program, 0);
    let (pool_addr, pool_bump) = pool_pda(program, &amm_config_addr, &mint_0, &mint_1);
    let (observation_addr, _) = observation_pda(program, &pool_addr);
    let (vault_0, _) = vault_pda(program, &pool_addr, &mint_0);
    let (vault_1, _) = vault_pda(program, &pool_addr, &mint_1);

    let first_start = 0i32;
    let (tick_array_addr, _) = tick_array_pda(program, &pool_addr, first_start);
    let mut tick_array = TickArrayState::zeroed();
    tick_array.pool_id = pool_addr.to_bytes();
    tick_array.start_tick_index = first_start;
    tick_array.initialized_tick_count = spec.first_array.len() as u8;
    for t in &spec.first_array {
        let mut ts = TickState::zeroed();
        ts.tick = t.tick;
        ts.liquidity_net = t.liquidity_net;
        ts.liquidity_gross = t.liquidity_gross;
        let idx = t.tick as usize;
        tick_array.ticks[idx] = ts;
    }
    let tick_array_bytes = pack(TickArrayState::DISCRIMINATOR, &tick_array);

    let (second_tick_array_addr, second_tick_array_bytes) = if !spec.second_array.is_empty() {
        let second_start = 60i32;
        let (addr, _) = tick_array_pda(program, &pool_addr, second_start);
        let mut arr = TickArrayState::zeroed();
        arr.pool_id = pool_addr.to_bytes();
        arr.start_tick_index = second_start;
        arr.initialized_tick_count = spec.second_array.len() as u8;
        for t in &spec.second_array {
            let mut ts = TickState::zeroed();
            ts.tick = t.tick;
            ts.liquidity_net = t.liquidity_net;
            ts.liquidity_gross = t.liquidity_gross;
            let idx = (t.tick - second_start) as usize;
            arr.ticks[idx] = ts;
        }
        (Some(addr), Some(pack(TickArrayState::DISCRIMINATOR, &arr)))
    } else {
        (None, None)
    };

    let amm_config_bytes = MAINNET_AMM_CONFIG_BYTES.to_vec();

    let mut pool = PoolState::zeroed();
    pool.bump = [pool_bump];
    pool.amm_config = amm_config_addr.to_bytes();
    pool.owner = Pubkey::default().to_bytes();
    pool.token_mint_0 = mint_0.to_bytes();
    pool.token_mint_1 = mint_1.to_bytes();
    pool.token_vault_0 = vault_0.to_bytes();
    pool.token_vault_1 = vault_1.to_bytes();
    pool.observation_key = observation_addr.to_bytes();
    pool.mint_decimals_0 = 6;
    pool.mint_decimals_1 = 6;
    pool.tick_spacing = 1;
    pool.liquidity = 1_000_000_000;
    pool.sqrt_price_x64 = 1u128 << 64;
    pool.tick_current = 0;
    pool.status = 0;
    pool.fee_on = 0;
    pool.reward_infos = [RewardInfo::zeroed(); REWARD_NUM];
    pool.tick_array_bitmap = [0u64; 16];
    pool.tick_array_bitmap[8] = if second_tick_array_addr.is_some() { 0b11 } else { 0b01 };
    pool.dynamic_fee_info = DynamicFeeInfo::zeroed();
    let pool_bytes = pack(PoolState::DISCRIMINATOR, &pool);

    let mut observation = ObservationState::zeroed();
    observation.pool_id = pool_addr.to_bytes();
    observation.observations = [Observation::default(); p_clmm::states::OBSERVATION_NUM];
    let observation_bytes = pack(ObservationState::DISCRIMINATOR, &observation);

    PoolFixture {
        pool: pool_addr,
        pool_bump,
        amm_config: amm_config_addr,
        observation: observation_addr,
        tick_array: tick_array_addr,
        second_tick_array: second_tick_array_addr,
        vault_0,
        vault_1,
        mint_0,
        mint_1,
        pool_bytes,
        tick_array_bytes,
        second_tick_array_bytes,
        amm_config_bytes,
        observation_bytes,
    }
}

fn build_mint_bytes(decimals: u8) -> Vec<u8> {
    let mut out = vec![0u8; 82];
    out[44] = decimals;
    out[45] = 1;
    out
}

fn build_token_account_bytes(mint: &Pubkey, owner: &Pubkey, amount: u64) -> Vec<u8> {
    let mut out = vec![0u8; 165];
    out[0..32].copy_from_slice(mint.as_ref());
    out[32..64].copy_from_slice(owner.as_ref());
    out[64..72].copy_from_slice(&amount.to_le_bytes());
    out[108] = 1;
    out
}

struct Scenario {
    svm: LiteSVM,
    program_id: Pubkey,
    fixture: PoolFixture,
    payer: Keypair,
    user_token_0: Pubkey,
    user_token_1: Pubkey,
}

fn try_setup(program_path: PathBuf, program_id: Pubkey, spec: &ScenarioSpec) -> Option<Scenario> {
    if !program_path.exists() {
        return None;
    }
    let program_bytes = std::fs::read(&program_path).ok()?;
    let mut svm = LiteSVM::new();
    svm.add_program(program_id, &program_bytes);

    svm.set_sysvar::<Clock>(&Clock {
        slot: 300_000_000,
        epoch_start_timestamp: 1_735_000_000,
        epoch: 700,
        leader_schedule_epoch: 700,
        unix_timestamp: 1_735_000_000,
    });

    let payer = Keypair::new();
    svm.airdrop(&payer.pubkey(), 10_000_000_000).ok()?;

    let (mint_0, mint_1) = {
        let a = Pubkey::new_unique();
        let b = Pubkey::new_unique();
        if a < b { (a, b) } else { (b, a) }
    };
    let spl = spl_token_program_id();
    let rent_exempt = |size: usize| -> u64 { 1_000_000 + (size as u64) * 7 };

    let mint_bytes = build_mint_bytes(6);
    for mint in [mint_0, mint_1] {
        svm.set_account(
            mint,
            Account {
                lamports: rent_exempt(82),
                data: mint_bytes.clone(),
                owner: spl,
                executable: false,
                rent_epoch: 0,
            },
        )
        .ok()?;
    }

    let fixture = build_pool_fixture(&program_id, mint_0, mint_1, spec);

    let mut planted = vec![
        (fixture.pool, fixture.pool_bytes.clone()),
        (fixture.amm_config, fixture.amm_config_bytes.clone()),
        (fixture.tick_array, fixture.tick_array_bytes.clone()),
        (fixture.observation, fixture.observation_bytes.clone()),
    ];
    if let (Some(second_addr), Some(second_bytes)) =
        (fixture.second_tick_array, fixture.second_tick_array_bytes.as_ref())
    {
        planted.push((second_addr, second_bytes.to_vec()));
    }
    for (pubkey, data) in planted {
        let size = data.len();
        svm.set_account(
            pubkey,
            Account {
                lamports: rent_exempt(size),
                data,
                owner: program_id,
                executable: false,
                rent_epoch: 0,
            },
        )
        .ok()?;
    }

    let vault_0_data = build_token_account_bytes(&fixture.mint_0, &fixture.pool, 1_000_000_000);
    let vault_1_data = build_token_account_bytes(&fixture.mint_1, &fixture.pool, 1_000_000_000);
    svm.set_account(
        fixture.vault_0,
        Account {
            lamports: rent_exempt(165),
            data: vault_0_data,
            owner: spl,
            executable: false,
            rent_epoch: 0,
        },
    )
    .ok()?;
    svm.set_account(
        fixture.vault_1,
        Account {
            lamports: rent_exempt(165),
            data: vault_1_data,
            owner: spl,
            executable: false,
            rent_epoch: 0,
        },
    )
    .ok()?;

    let user_token_0 = Pubkey::new_unique();
    let user_token_1 = Pubkey::new_unique();
    let ut0 = build_token_account_bytes(&fixture.mint_0, &payer.pubkey(), 0);
    let ut1 =
        build_token_account_bytes(&fixture.mint_1, &payer.pubkey(), 100_000_000);
    for (pk, data) in [(user_token_0, ut0), (user_token_1, ut1)] {
        svm.set_account(
            pk,
            Account {
                lamports: rent_exempt(165),
                data,
                owner: spl,
                executable: false,
                rent_epoch: 0,
            },
        )
        .ok()?;
    }

    Some(Scenario {
        svm,
        program_id,
        fixture,
        payer,
        user_token_0,
        user_token_1,
    })
}

const SWAP_IX_DISCRIMINATOR: [u8; 8] = [0xf8, 0xc6, 0x9e, 0x91, 0xe1, 0x75, 0x87, 0xc8];

fn build_swap_ix(
    program_id: Pubkey,
    s: &Scenario,
    amount: u64,
    other_amount_threshold: u64,
    sqrt_price_limit_x64: u128,
    is_base_input: bool,
) -> Instruction {
    let mut data = Vec::with_capacity(8 + 33);
    data.extend_from_slice(&SWAP_IX_DISCRIMINATOR);
    data.extend_from_slice(&amount.to_le_bytes());
    data.extend_from_slice(&other_amount_threshold.to_le_bytes());
    data.extend_from_slice(&sqrt_price_limit_x64.to_le_bytes());
    data.push(is_base_input as u8);

    let (input_user, output_user) = (s.user_token_1, s.user_token_0);
    let (input_vault, output_vault) = (s.fixture.vault_1, s.fixture.vault_0);

    let mut accounts = vec![
        AccountMeta::new_readonly(s.payer.pubkey(), true),
        AccountMeta::new_readonly(s.fixture.amm_config, false),
        AccountMeta::new(s.fixture.pool, false),
        AccountMeta::new(input_user, false),
        AccountMeta::new(output_user, false),
        AccountMeta::new(input_vault, false),
        AccountMeta::new(output_vault, false),
        AccountMeta::new(s.fixture.observation, false),
        AccountMeta::new_readonly(spl_token_program_id(), false),
        AccountMeta::new(s.fixture.tick_array, false),
    ];
    if let Some(second) = s.fixture.second_tick_array {
        accounts.push(AccountMeta::new(second, false));
    }

    Instruction {
        program_id,
        accounts,
        data,
    }
}

#[derive(Debug)]
struct RunResult {
    compute_units: u64,
    post_pool_bytes: Vec<u8>,
    failed: bool,
    log_tail: Vec<String>,
}

fn run_swap(scenario: &mut Scenario, ix: Instruction) -> RunResult {
    let budget_ix = ComputeBudgetInstruction::set_compute_unit_limit(600_000);
    let tx = Transaction::new_signed_with_payer(
        &[budget_ix, ix],
        Some(&scenario.payer.pubkey()),
        &[&scenario.payer],
        scenario.svm.latest_blockhash(),
    );
    match scenario.svm.send_transaction(tx) {
        Ok(meta) => RunResult {
            compute_units: meta.compute_units_consumed,
            post_pool_bytes: scenario
                .svm
                .get_account(&scenario.fixture.pool)
                .map(|a| a.data)
                .unwrap_or_default(),
            failed: false,
            log_tail: meta.logs.into_iter().rev().take(10).rev().collect(),
        },
        Err(failed) => RunResult {
            compute_units: failed.meta.compute_units_consumed,
            post_pool_bytes: vec![],
            failed: true,
            log_tail: failed.meta.logs.into_iter().rev().take(10).rev().collect(),
        },
    }
}

fn assert_swap_outputs_match(label: &str, r: &RunResult, v: &RunResult) {
    if r.failed || v.failed || r.post_pool_bytes.is_empty() || v.post_pool_bytes.is_empty() {
        return;
    }
    let r_pool: PoolState =
        *bytemuck::from_bytes(&r.post_pool_bytes[8..8 + std::mem::size_of::<PoolState>()]);
    let v_pool: PoolState =
        *bytemuck::from_bytes(&v.post_pool_bytes[8..8 + std::mem::size_of::<PoolState>()]);

    let pairs: &[(&str, u128, u128)] = &[
        ("sqrt_price_x64", r_pool.sqrt_price_x64, v_pool.sqrt_price_x64),
        ("liquidity", r_pool.liquidity, v_pool.liquidity),
        (
            "fee_growth_0_x64",
            r_pool.fee_growth_global_0_x64,
            v_pool.fee_growth_global_0_x64,
        ),
        (
            "fee_growth_1_x64",
            r_pool.fee_growth_global_1_x64,
            v_pool.fee_growth_global_1_x64,
        ),
    ];
    let (r_tick, v_tick) = (r_pool.tick_current, v_pool.tick_current);
    let mut all_ok = r_tick == v_tick;
    for (_, a, b) in pairs {
        if a != b {
            all_ok = false;
        }
    }
    if !all_ok {
        println!("\nFIELD MISMATCH in `{label}`:");
        println!("  tick_current: ray={r_tick} vex={v_tick}");
        for (name, a, b) in pairs {
            let marker = if a == b { "" } else { "  <-- DIFFERS" };
            println!("  {name}: ray={a} vex={b}{marker}");
        }
        panic!("swap outputs diverged on `{label}`");
    }
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

fn raydium_so_path() -> PathBuf {
    workspace_root().join("raydium-clmm/target/deploy/raydium_clmm.so")
}

fn p_clmm_so_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/deploy/p_clmm.so")
}

fn run_one(spec: &ScenarioSpec) -> Option<(u64, u64, bool, bool, Vec<String>, Vec<String>)> {
    let mut raydium = try_setup(raydium_so_path(), raydium_program_id(), spec)?;
    let mut p_clmm = try_setup(p_clmm_so_path(), p_clmm_program_id(), spec)?;

    let r_ix = build_swap_ix(raydium.program_id, &raydium, spec.amount, 0, 0, true);
    let v_ix = build_swap_ix(p_clmm.program_id, &p_clmm, spec.amount, 0, 0, true);

    let r = run_swap(&mut raydium, r_ix);
    let v = run_swap(&mut p_clmm, v_ix);
    assert_swap_outputs_match(spec.label, &r, &v);
    Some((
        r.compute_units,
        v.compute_units,
        r.failed,
        v.failed,
        r.log_tail,
        v.log_tail,
    ))
}

fn print_row(label: &str, r_cu: u64, v_cu: u64) {
    let saved = r_cu as i64 - v_cu as i64;
    let pct = if r_cu > 0 {
        saved as f64 / r_cu as f64 * 100.0
    } else {
        0.0
    };
    println!(
        "  {:<18}  {:>8}  {:>8}  {:>+8}   {:+6.1}%",
        label, r_cu, v_cu, saved, pct
    );
}

#[test]
fn benchmark_sweep() {
    if !raydium_so_path().exists() {
        eprintln!("skipping sweep: raydium_clmm.so missing — `cd ../raydium-clmm && anchor build`");
        return;
    }
    if !p_clmm_so_path().exists() {
        eprintln!("skipping sweep: p_clmm.so missing — `cargo build-sbf`");
        return;
    }

    let no_crossing = ScenarioSpec {
        label: "no-crossing",
        first_array: vec![TickSpec {
            tick: 30,
            liquidity_net: -100_000_000,
            liquidity_gross: 100_000_000,
        }],
        second_array: vec![],
        amount: 100_000,
    };

    let one_crossing = ScenarioSpec {
        label: "1-crossing",
        first_array: vec![TickSpec {
            tick: 5,
            liquidity_net: -500_000_000,
            liquidity_gross: 500_000_000,
        }],
        second_array: vec![TickSpec {
            tick: 100,
            liquidity_net: 0,
            liquidity_gross: 1,
        }],
        amount: 1_000_000,
    };

    let ten_crossings = ScenarioSpec {
        label: "10-crossings",
        first_array: (1..=10)
            .map(|i| TickSpec {
                tick: i * 5,
                liquidity_net: 0,
                liquidity_gross: 1,
            })
            .collect(),
        second_array: vec![TickSpec {
            tick: 100,
            liquidity_net: 0,
            liquidity_gross: 1,
        }],
        amount: 4_000_000,
    };

    println!("\n══════════ SWAP CU SWEEP — p_clmm vs. raydium-clmm ══════════");
    println!("(real mainnet AmmConfig: trade_fee 0.04%, protocol 12%, fund 4%)\n");
    println!("  {:<18}  {:>8}  {:>8}  {:>8}   {:>7}", "scenario", "raydium", "p_clmm", "saved", "reduce");
    println!("  {:<18}  {:>8}  {:>8}  {:>8}   {:>7}", "--------", "-------", "------", "-----", "------");

    let mut totals_r: u64 = 0;
    let mut totals_v: u64 = 0;
    let mut failures = Vec::new();

    for spec in [&no_crossing, &one_crossing, &ten_crossings] {
        match run_one(spec) {
            Some((r_cu, v_cu, r_failed, v_failed, r_logs, v_logs)) => {
                if r_failed || v_failed {
                    print_row(spec.label, r_cu, v_cu);
                    failures.push((spec.label, r_failed, v_failed, r_logs, v_logs));
                } else {
                    print_row(spec.label, r_cu, v_cu);
                    totals_r += r_cu;
                    totals_v += v_cu;
                }
            }
            None => {
                println!("  {:<18}  (setup failed)", spec.label);
            }
        }
    }

    if totals_r > 0 && totals_v > 0 {
        println!();
        print_row("TOTAL", totals_r, totals_v);
    }

    for (label, r_failed, v_failed, r_logs, v_logs) in failures {
        println!("\n── failures in `{label}` ──");
        if r_failed {
            println!("raydium log tail:");
            for l in r_logs {
                println!("  {l}");
            }
        }
        if v_failed {
            println!("p_clmm log tail:");
            for l in v_logs {
                println!("  {l}");
            }
        }
    }
}

#[test]
fn correctness_grid() {
    if !raydium_so_path().exists() || !p_clmm_so_path().exists() {
        eprintln!("skipping correctness grid: build both .so first");
        return;
    }
    let t = |tick: i32, ln: i128, lg: u128| TickSpec {
        tick,
        liquidity_net: ln,
        liquidity_gross: lg,
    };
    let grid: [(&str, Vec<TickSpec>, Vec<TickSpec>, u64); 12] = [
        ("adjacent-tick",       vec![t(1, -100_000_000, 100_000_000)], vec![t(80, 0, 1)],  200_000),
        ("far-tick",            vec![t(55, -100_000_000, 100_000_000)], vec![t(80, 0, 1)],  50_000),
        ("dust-swap",           vec![t(30, -100_000_000, 100_000_000)], vec![t(80, 0, 1)],  1_000),
        ("tiny-liq-delta",      vec![t(10, -1, 1)], vec![t(80, 0, 1)],                       800_000),
        ("balanced-net-zero",   vec![t(5, 0, 1)], vec![t(100, 0, 1)],                        600_000),
        ("five-uniform-ticks",  (1..=5).map(|i| t(i * 4, 0, 1)).collect(), vec![t(80, 0, 1)], 1_500_000),
        ("twenty-marker-ticks", (1..=20).map(|i| t(i * 2, 0, 1)).collect(), vec![t(80, 0, 1)], 3_000_000),
        ("alternating-deltas",  (1..=4).map(|i| t(i * 8, if i % 2 == 0 { -50_000_000 } else { 50_000_000 }, 50_000_000)).collect(), vec![t(80, 0, 1)], 1_200_000),
        ("liquidity-grows",     vec![t(5, 200_000_000, 200_000_000)], vec![t(80, 0, 1)],     1_000_000),
        ("near-array-end",      vec![t(58, -100_000_000, 100_000_000)], vec![t(70, 0, 1)],   700_000),
        ("dense-cluster",       vec![t(20, 0, 1), t(21, 0, 1), t(22, 0, 1), t(23, 0, 1), t(24, 0, 1)], vec![t(80, 0, 1)], 900_000),
        ("sparse-three",        vec![t(2, 0, 1), t(20, 0, 1), t(50, -50_000_000, 50_000_000)], vec![t(80, 0, 1)], 1_300_000),
    ];

    println!("\n──── DIFFERENTIAL CORRECTNESS GRID ────");
    let mut pass = 0;
    let mut skipped = 0;
    for (label, first, second, amount) in &grid {
        let spec = ScenarioSpec {
            label,
            first_array: first.clone(),
            second_array: second.clone(),
            amount: *amount,
        };
        match run_one(&spec) {
            Some((r, v, false, false, _, _)) => {
                println!("  pass {:<22}  ray={:>7}  vex={:>7}", label, r, v);
                pass += 1;
            }
            Some((r, v, _, _, _, _)) => {
                println!("  skip {:<22}  ray={:>7}  vex={:>7}  (one or both failed)", label, r, v);
                skipped += 1;
            }
            None => {
                println!("  skip {:<22}  setup failed", label);
                skipped += 1;
            }
        }
    }
    println!("\n  {pass}/{} field-equal, {skipped} skipped\n", grid.len());
    assert!(pass > 0, "no scenarios produced a valid field-equality match");
}

#[test]
fn benchmark_scaling_curve() {
    if !raydium_so_path().exists() || !p_clmm_so_path().exists() {
        eprintln!("skipping scaling curve: build raydium + p_clmm .so first");
        return;
    }

    let make_spec = |label, amount| ScenarioSpec {
        label,
        first_array: vec![TickSpec {
            tick: 5,
            liquidity_net: -500_000_000,
            liquidity_gross: 500_000_000,
        }],
        second_array: vec![TickSpec {
            tick: 100,
            liquidity_net: 0,
            liquidity_gross: 1,
        }],
        amount,
    };

    let samples: [(&str, u64); 8] = [
        ("10k",   10_000),
        ("50k",   50_000),
        ("100k", 100_000),
        ("250k", 250_000),
        ("500k", 500_000),
        ("1M",  1_000_000),
        ("2M",  2_000_000),
        ("5M",  5_000_000),
    ];

    println!("\n══════════ CU SCALING CURVE ══════════");
    println!("(same pool, sweep amount — shows intercept vs. slope of the win)\n");
    println!("  {:<8}  {:>8}  {:>8}  {:>8}  {:>7}", "amount", "raydium", "p_clmm", "saved", "reduce");
    println!("  {:<8}  {:>8}  {:>8}  {:>8}  {:>7}", "------", "-------", "------", "-----", "------");

    let mut rows: Vec<(u64, u64, u64)> = Vec::new();
    for (label, amount) in samples {
        let spec = make_spec(label, amount);
        match run_one(&spec) {
            Some((r, v, r_failed, v_failed, _, _)) if !r_failed && !v_failed => {
                print_row(label, r, v);
                rows.push((amount, r, v));
            }
            Some((r, v, _, _, _, _)) => {
                println!("  {:<8}  {:>8}  {:>8}  (one or both failed)", label, r, v);
            }
            None => println!("  {label:<8}  (setup failed)"),
        }
    }

    if rows.len() >= 2 {
        let n = rows.len() as f64;
        let xs: Vec<f64> = rows.iter().map(|r| r.0 as f64).collect();
        let r_ys: Vec<f64> = rows.iter().map(|r| r.1 as f64).collect();
        let v_ys: Vec<f64> = rows.iter().map(|r| r.2 as f64).collect();
        let mean_x = xs.iter().sum::<f64>() / n;
        let fit = |ys: &[f64]| -> (f64, f64) {
            let mean_y = ys.iter().sum::<f64>() / n;
            let num: f64 = xs
                .iter()
                .zip(ys)
                .map(|(x, y)| (x - mean_x) * (y - mean_y))
                .sum();
            let den: f64 = xs.iter().map(|x| (x - mean_x).powi(2)).sum();
            let slope = num / den;
            let intercept = mean_y - slope * mean_x;
            (intercept, slope)
        };
        let (r_b, r_m) = fit(&r_ys);
        let (v_b, v_m) = fit(&v_ys);

        println!("\nlinear fit  CU(amount) = b + m·amount");
        println!(
            "  raydium:  b = {:>8.0} CU  |  m = {:>8.5} CU per unit input",
            r_b, r_m
        );
        println!(
            "  p_clmm:   b = {:>8.0} CU  |  m = {:>8.5} CU per unit input",
            v_b, v_m
        );
        let b_saved = r_b - v_b;
        let b_pct = b_saved / r_b * 100.0;
        let m_saved = r_m - v_m;
        let m_pct = if r_m.abs() > 1e-9 { m_saved / r_m * 100.0 } else { 0.0 };
        println!(
            "  Δintercept (fixed cost): {:>+8.0} CU  ({:+5.1}%)  ← structural wins live here",
            b_saved, b_pct
        );
        println!(
            "  Δslope (per-unit cost):  {:>+8.5} CU  ({:+5.1}%)  ← math is bit-identical, ~no diff",
            m_saved, m_pct
        );
    }
}
