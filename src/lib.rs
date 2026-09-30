#![no_std]
extern crate alloc;
use pinocchio::{AccountView, Address, ProgramResult, program_entrypoint, default_allocator, nostd_panic_handler};
use pinocchio::error::ProgramError;
program_entrypoint!(process_instruction);
default_allocator!();
nostd_panic_handler!();
mod ops;

pub const CONFIG_LEN: usize = 576;
pub const STORE_LEN: usize = 256;
pub const ORDER_LEN: usize = 128;
pub const TOKEN_2022: Address = Address::from_str_const("TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb");
pub const CORE: Address = Address::from_str_const("CoREENxT6tW1HoK8ypY1SxRMZTcVPm7R94rH4PZNhX7d");
pub const JUPITER: Address = Address::from_str_const("JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4");
pub const SYSTEM: Address = Address::from_str_const("11111111111111111111111111111111");
pub const LOADER: Address = Address::from_str_const("BPFLoaderUpgradeab1e11111111111111111111111");

pub mod c {
    pub const CTOWN_PROGRAM: usize = 512;
    pub const ADMIN: usize = 2; pub const PENDING: usize = 34; pub const OPERATOR: usize = 66;
    pub const PAUSER: usize = 98; pub const TREASURY: usize = 130; pub const USDC: usize = 162;
    pub const CTOWN: usize = 194; pub const COLLECTION: usize = 226; pub const URI: usize = 258;
    pub const URI_LEN: usize = 322; pub const MINT_PRICE: usize = 324; pub const SPLIT: usize = 332;
    pub const FEE_SALE: usize = 338; pub const FEE_ORDER: usize = 340; pub const MAX_TEAMS: usize = 342;
    pub const MINTED: usize = 344; pub const THRESH: usize = 346; pub const CAPACITY: usize = 378;
    pub const COOLDOWN: usize = 386; pub const REVIEW: usize = 390; pub const REWARD_CAP: usize = 394;
    pub const REWARD_DAY: usize = 396; pub const REWARD_PAID: usize = 404; pub const REWARD_BASE: usize = 412;
    pub const GLOBAL_CAP: usize = 420; pub const GLOBAL_DAY: usize = 428; pub const GLOBAL_SPENT: usize = 436;
    pub const FEES: usize = 444; pub const LIABILITIES: usize = 452; pub const TOTALS: usize = 460;
    pub const BURN: usize = 508; pub const PAUSED: usize = 509; pub const BUMP: usize = 510;
    // v1.1: one-way wind-down flag and count of closed stores (config grew 544 -> 576)
    pub const WIND: usize = 511; pub const CLOSED: usize = 544;
}
pub mod s {
    pub const TEAM: usize = 2; pub const BUMP: usize = 4; pub const FOUNDING: usize = 5;
    pub const CREATED: usize = 6; pub const BAL: usize = 14; pub const ESCROW: usize = 22;
    pub const REV_DAY: usize = 30; pub const REV_CUR: usize = 38; pub const REV_PREV: usize = 46;
    pub const TRADE: usize = 54; pub const DAILY: usize = 62; pub const RESERVE: usize = 70;
    pub const AGENTS_PAUSED: usize = 78; pub const SPEND_DAY: usize = 79; pub const SPENT: usize = 87;
    pub const SEQ: usize = 95; pub const BOND: usize = 103; pub const UNBOND: usize = 111;
    pub const UNBOND_AT: usize = 119; pub const ACTIVE: usize = 127; pub const COUNTERS: usize = 129;
}
pub mod o {
    pub const ID: usize = 2; pub const BUYER: usize = 10; pub const SELLER: usize = 12;
    pub const AMOUNT: usize = 14; pub const FEE: usize = 22; pub const CREATED: usize = 24;
    pub const DEADLINE: usize = 32; pub const DELIVERED: usize = 40; pub const REVIEW: usize = 48;
    pub const STATUS: usize = 56; pub const HASH: usize = 57; pub const PAYER: usize = 89;
}

pub fn err(n:u32)->ProgramError{ProgramError::Custom(n)}
pub fn need(ok:bool,n:u32)->ProgramResult{if ok{Ok(())}else{Err(err(n))}}
pub fn add(a:u64,b:u64)->Result<u64,ProgramError>{a.checked_add(b).ok_or(err(7))}
pub fn sub(a:u64,b:u64)->Result<u64,ProgramError>{a.checked_sub(b).ok_or(err(7))}
pub fn mul(a:u64,b:u64)->Result<u64,ProgramError>{a.checked_mul(b).ok_or(err(7))}
pub fn x8(d:&[u8],i:usize)->Result<u64,ProgramError>{Ok(u64::from_le_bytes(d.get(i..i+8).ok_or(err(2))?.try_into().map_err(|_|err(2))?))}
pub fn x16(d:&[u8],i:usize)->Result<u16,ProgramError>{Ok(u16::from_le_bytes(d.get(i..i+2).ok_or(err(2))?.try_into().map_err(|_|err(2))?))}
pub fn xi(d:&[u8],i:usize)->Result<i64,ProgramError>{Ok(i64::from_le_bytes(d.get(i..i+8).ok_or(err(2))?.try_into().map_err(|_|err(2))?))}
pub fn w8(d:&mut[u8],i:usize,v:u64){d[i..i+8].copy_from_slice(&v.to_le_bytes())}
pub fn w16(d:&mut[u8],i:usize,v:u16){d[i..i+2].copy_from_slice(&v.to_le_bytes())}
pub fn wi(d:&mut[u8],i:usize,v:i64){d[i..i+8].copy_from_slice(&v.to_le_bytes())}
pub fn inc(d:&mut[u8],i:usize,n:u64)->ProgramResult{let v=add(x8(d,i)?,n)?;w8(d,i,v);Ok(())}
pub fn dec(d:&mut[u8],i:usize,n:u64)->ProgramResult{let v=sub(x8(d,i)?,n)?;w8(d,i,v);Ok(())}
pub fn inc16(d:&mut[u8],i:usize)->ProgramResult{let v=x16(d,i)?.checked_add(1).ok_or(err(7))?;w16(d,i,v);Ok(())}
pub fn dec16(d:&mut[u8],i:usize)->ProgramResult{let v=x16(d,i)?.checked_sub(1).ok_or(err(7))?;w16(d,i,v);Ok(())}
pub fn key(d:&[u8],i:usize)->Result<Address,ProgramError>{Ok(Address::new_from_array(d.get(i..i+32).ok_or(err(2))?.try_into().map_err(|_|err(2))?))}
pub fn wk(d:&mut[u8],i:usize,a:&Address){d[i..i+32].copy_from_slice(a.as_ref())}
pub fn account(a:&[AccountView],i:usize)->Result<&AccountView,ProgramError>{a.get(i).ok_or(err(1))}
pub fn sign(a:&AccountView)->ProgramResult{need(a.is_signer(),3)}
pub fn write(a:&AccountView)->ProgramResult{need(a.is_writable(),4)}
pub fn pda(pid:&Address,seeds:&[&[u8]],a:&AccountView)->Result<u8,ProgramError>{let(k,b)=Address::find_program_address(seeds,pid);need(a.address()==&k,5)?;Ok(b)}
pub fn read<const N:usize>(a:&AccountView,pid:&Address,disc:u8)->Result<[u8;N],ProgramError>{
    need(a.owned_by(pid)&&a.data_len()==N,5)?;let d=a.try_borrow()?;need(d[0]==disc&&d[1]==1,2)?;
    let mut out=[0;N];out.copy_from_slice(&d);Ok(out)
}
pub fn save<const N:usize>(a:&mut AccountView,d:&[u8;N])->ProgramResult{write(a)?;a.try_borrow_mut()?.copy_from_slice(d);Ok(())}
pub fn config(a:&AccountView,pid:&Address)->Result<[u8;CONFIG_LEN],ProgramError>{pda(pid,&[b"config"],a)?;read(a,pid,1)}
pub fn store(a:&AccountView,pid:&Address)->Result<[u8;STORE_LEN],ProgramError>{
    let d=read::<STORE_LEN>(a,pid,2)?;let id=&d[s::TEAM..s::TEAM+2];need(pda(pid,&[b"store",id],a)?==d[s::BUMP],5)?;Ok(d)
}
pub fn order(a:&AccountView,pid:&Address)->Result<[u8;ORDER_LEN],ProgramError>{let d=read::<ORDER_LEN>(a,pid,3)?;pda(pid,&[b"order",&d[o::ID..o::ID+8]],a)?;Ok(d)}
pub fn owner(asset:&AccountView,pid:&Address,team:u16,signer:&AccountView)->ProgramResult{
    sign(signer)?;pda(pid,&[b"agent",&team.to_le_bytes(),&[0]],asset)?;
    need(asset.owned_by(&CORE)&&asset.data_len()>=33,5)?;
    let d=asset.try_borrow()?;need(d[0]==1&&&d[1..33]==signer.address().as_ref(),3)
}
pub fn token(a:&AccountView,mint:&Address,auth:&Address)->Result<u64,ProgramError>{
    need(a.owned_by(&pinocchio_token::ID)&&a.data_len()==165,5)?;
    let d=a.try_borrow()?;need(&d[0..32]==mint.as_ref()&&&d[32..64]==auth.as_ref()&&d[108]==1,5)?;x8(&d,64)
}
pub fn now()->Result<i64,ProgramError>{use pinocchio::sysvars::Sysvar;Ok(pinocchio::sysvars::clock::Clock::get()?.unix_timestamp)}
pub fn ctown_token(a:&AccountView,mint:&Address,auth:&Address,program:&Address)->Result<u64,ProgramError>{
    need(a.owned_by(program)&&a.data_len()>=165,5)?;
    let d=a.try_borrow()?;need(&d[0..32]==mint.as_ref()&&&d[32..64]==auth.as_ref()&&d[108]==1,5)?;x8(&d,64)
}
pub fn day(t:i64)->i64{t.div_euclid(86400)}
pub fn emit(tag:u8,fields:&[u8]){
    #[cfg(target_os="solana")]
    {use alloc::vec::Vec;let mut d=Vec::with_capacity(fields.len()+1);d.push(tag);d.extend_from_slice(fields);let raw=[(d.as_ptr(),d.len() as u64)];unsafe{pinocchio::syscalls::sol_log_data(raw.as_ptr() as *const u8,1)};}
    #[cfg(not(target_os="solana"))]{let _=(tag,fields);}
}
pub fn process_instruction(pid:&Address,a:&mut[AccountView],d:&[u8])->ProgramResult{let op=*d.first().ok_or(err(1))?;ops::dispatch(op,pid,a,&d[1..])}
