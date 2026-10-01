use alloc::vec::Vec;
use pinocchio::{AccountView,Address,ProgramResult};
use pinocchio::error::ProgramError;
use pinocchio::cpi::{Seed,Signer,invoke_signed,invoke_signed_with_slice};
use pinocchio::instruction::{InstructionAccount,InstructionView};
use pinocchio_system::instructions::{CreateAccount,Transfer};
use crate::*;

fn acc(a:&[AccountView],i:usize)->Result<&AccountView,ProgramError>{account(a,i)}
fn cfg(a:&[AccountView],pid:&Address)->Result<[u8;CONFIG_LEN],ProgramError>{config(acc(a,0)?,pid)}
fn auth(d:&[u8],i:usize,a:&AccountView)->ProgramResult{sign(a)?;need(a.address()==&key(d,i)?,3)}
fn vault(pid:&Address,a:&AccountView)->Result<u8,ProgramError>{pda(pid,&[b"vault"],a)}
fn tpda(pid:&Address,name:&[u8],a:&AccountView,mint:&Address,v:&Address)->Result<u64,ProgramError>{pda(pid,&[name],a)?;token(a,mint,v)}
fn raw_token(a:&AccountView,mint:&Address)->Result<u64,ProgramError>{need(a.owned_by(&pinocchio_token::ID)&&a.data_len()==165,5)?;let d=a.try_borrow()?;need(&d[0..32]==mint.as_ref()&&d[108]==1,5)?;x8(&d,64)}
fn amount(a:&AccountView)->Result<u64,ProgramError>{x8(&a.try_borrow()?,64)}
fn check_liab(c:&[u8;CONFIG_LEN],store_vault:&AccountView)->ProgramResult{need(amount(store_vault)?>=x8(c,c::LIABILITIES)?,8)}
fn bump_signer<'a>(name:&'a[u8],b:&'a[u8;1])->[Seed<'a>;2]{[Seed::from(name),Seed::from(b)]}
fn transfer(from:&AccountView,to:&AccountView,authority:&AccountView,n:u64,signers:&[Signer])->ProgramResult{
    let data={let mut d=[0u8;9];d[0]=3;d[1..9].copy_from_slice(&n.to_le_bytes());d};
    let metas=[InstructionAccount::writable(from.address()),InstructionAccount::writable(to.address()),InstructionAccount::readonly_signer(authority.address())];
    let ix=InstructionView{program_id:&pinocchio_token::ID,accounts:&metas,data:&data};
    invoke_signed(&ix,&[from,to,authority],signers)
}
fn ctown_mint(c:&[u8;CONFIG_LEN],mint:&AccountView,program:&AccountView)->Result<u8,ProgramError>{
    need(program.address()==&key(c,c::CTOWN_PROGRAM)?&&(program.address()==&pinocchio_token::ID||program.address()==&TOKEN_2022),5)?;
    need(mint.address()==&key(c,c::CTOWN)?&&mint.owned_by(program.address())&&mint.data_len()>=82,5)?;
    let d=mint.try_borrow()?;need(d[45]==1,5)?;Ok(d[44])
}
fn transfer_checked(from:&AccountView,mint:&AccountView,to:&AccountView,authority:&AccountView,program:&Address,decimals:u8,n:u64,signers:&[Signer])->ProgramResult{
    let mut data=[0u8;10];data[0]=12;data[1..9].copy_from_slice(&n.to_le_bytes());data[9]=decimals;
    let metas=[InstructionAccount::writable(from.address()),InstructionAccount::readonly(mint.address()),InstructionAccount::writable(to.address()),InstructionAccount::readonly_signer(authority.address())];
    invoke_signed(&InstructionView{program_id:program,accounts:&metas,data:&data},&[from,mint,to,authority],signers)
}
fn burn(account:&AccountView,mint:&AccountView,authority:&AccountView,program:&Address,decimals:u8,n:u64,signers:&[Signer])->ProgramResult{
    let mut data=[0u8;10];data[0]=15;data[1..9].copy_from_slice(&n.to_le_bytes());data[9]=decimals;
    let metas=[InstructionAccount::writable(account.address()),InstructionAccount::writable(mint.address()),InstructionAccount::readonly_signer(authority.address())];
    invoke_signed(&InstructionView{program_id:program,accounts:&metas,data:&data},&[account,mint,authority],signers)
}
fn ctown_account_size(mint:&AccountView,program:&Address)->Result<usize,ProgramError>{
    let metas=[InstructionAccount::readonly(mint.address())];
    invoke_signed(&InstructionView{program_id:program,accounts:&metas,data:&[21]},&[mint],&[])?;
    #[cfg(target_os="solana")]
    {
        let mut data=[0u8;8];let mut owner=[0u8;32];
        let len=unsafe{pinocchio::syscalls::sol_get_return_data(data.as_mut_ptr(),8,owner.as_mut_ptr())};
        need(len==8&&owner.as_slice()==program.as_ref(),5)?;
        let size=usize::try_from(u64::from_le_bytes(data)).map_err(|_|err(7))?;
        need(size>=165,5)?;Ok(size)
    }
    #[cfg(not(target_os="solana"))]
    {Err(err(5))}
}
fn create_ctown_token(pid:&Address,payer:&AccountView,new:&AccountView,name:&[u8],mint:&AccountView,vault:&AccountView,program:&Address,size:usize)->ProgramResult{
    create(pid,payer,new,name,None,size,program)?;
    let mut data=[0u8;33];data[0]=18;data[1..].copy_from_slice(vault.address().as_ref());
    let metas=[InstructionAccount::writable(new.address()),InstructionAccount::readonly(mint.address())];
    invoke_signed(&InstructionView{program_id:program,accounts:&metas,data:&data},&[new,mint],&[])
}
fn create(pid:&Address,payer:&AccountView,new:&AccountView,name:&[u8],extra:Option<&[u8]>,len:usize,owner:&Address)->ProgramResult{
    let mut seeds=Vec::new();seeds.push(Seed::from(name));if let Some(v)=extra{seeds.push(Seed::from(v));}
    let two=[name,extra.unwrap_or(b"")];let bump=pda(pid,if extra.is_some(){&two[..]}else{&two[..1]},new)?;
    let b=[bump];seeds.push(Seed::from(&b));
    unsquat(new,payer,&[Signer::from(seeds.as_slice())])?;
    CreateAccount::with_minimum_balance(payer,new,len as u64,owner,None)?.invoke_signed(&[Signer::from(seeds.as_slice())])
}
// Anyone can send lamports to a PDA before it exists, and CreateAccount then refuses it for good (mint_team, orders,
// set_ctown_mint). The PDA is still system-owned and empty (only its seeds can allocate or assign it): hand the
// lamports to the payer, signed by the PDA, so the create goes through.
fn unsquat(new:&AccountView,payer:&AccountView,signers:&[Signer])->ProgramResult{
    let n=new.lamports();if n==0{return Ok(());}
    need(new.owned_by(&SYSTEM)&&new.is_data_empty(),5)?;
    Transfer{from:new,to:payer,lamports:n}.invoke_signed(signers)
}
fn create_token(pid:&Address,payer:&AccountView,new:&AccountView,name:&[u8],mint:&AccountView,vault:&AccountView)->ProgramResult{
    create(pid,payer,new,name,None,165,&pinocchio_token::ID)?;
    pinocchio_token::instructions::InitializeAccount3::new(new,mint,vault.address()).invoke()
}
fn flag(v:u8)->ProgramResult{need(v<=1,2)}
fn fee(n:u64,bps:u16)->Result<u64,ProgramError>{Ok(mul(n,bps as u64)?/10000)}
fn tier(c:&[u8;CONFIG_LEN],bond:u64)->usize{let mut t=0;for i in 1..4{if bond>=x8(c,c::THRESH+i*8).unwrap_or(u64::MAX){t=i;}}t}
fn capacity(c:&[u8;CONFIG_LEN],bond:u64)->Result<u16,ProgramError>{x16(c,c::CAPACITY+tier(c,bond)*2)}
fn active(c:&[u8;CONFIG_LEN],st:&[u8;STORE_LEN])->ProgramResult{need(x8(st,s::BOND)?>=x8(c,c::THRESH)?,9)}
fn roll_revenue(st:&mut[u8;STORE_LEN],today:i64)->ProgramResult{
    let old=xi(st,s::REV_DAY)?;
    if old!=today{let prev=if old==today-1{x8(st,s::REV_CUR)?}else{0};w8(st,s::REV_PREV,prev);w8(st,s::REV_CUR,0);wi(st,s::REV_DAY,today);}Ok(())
}
fn credit(st:&mut[u8;STORE_LEN],n:u64,today:i64)->ProgramResult{roll_revenue(st,today)?;inc(st,s::BAL,n)?;inc(st,s::REV_CUR,n)}
fn spend(c:&mut[u8;CONFIG_LEN],b:&mut[u8;STORE_LEN],n:u64,seq:u64,today:i64)->ProgramResult{
    need(c[c::PAUSED]==0&&b[s::AGENTS_PAUSED]==0,10)?;active(c,b)?;
    need(n>0&&n<=x8(b,s::TRADE)?&&seq>x8(b,s::SEQ)?,9)?;
    if xi(b,s::SPEND_DAY)?!=today{wi(b,s::SPEND_DAY,today);w8(b,s::SPENT,0);}
    if xi(c,c::GLOBAL_DAY)?!=today{wi(c,c::GLOBAL_DAY,today);w8(c,c::GLOBAL_SPENT,0);}
    need(add(x8(b,s::SPENT)?,n)?<=x8(b,s::DAILY)?&&add(x8(c,c::GLOBAL_SPENT)?,n)?<=x8(c,c::GLOBAL_CAP)?,9)?;
    need(sub(x8(b,s::BAL)?,n)?>=x8(b,s::RESERVE)?,9)?;
    dec(b,s::BAL,n)?;inc(b,s::SPENT,n)?;w8(b,s::SEQ,seq);
    inc(c,c::GLOBAL_SPENT,n)
}
fn counter(st:&mut[u8;STORE_LEN],index:usize,amount:u64)->ProgramResult{inc(st,s::COUNTERS+index*8,amount)}
fn close_order(order:&mut AccountView,payer:&mut AccountView)->ProgramResult{
    write(order)?;write(payer)?;let lam=order.lamports();payer.set_lamports(add(payer.lamports(),lam)?);order.set_lamports(0);order.close()
}
fn close_token(acct:&AccountView,dest:&AccountView,auth:&AccountView,program:&Address,signers:&[Signer])->ProgramResult{
    let metas=[InstructionAccount::writable(acct.address()),InstructionAccount::writable(dest.address()),InstructionAccount::readonly_signer(auth.address())];
    invoke_signed(&InstructionView{program_id:program,accounts:&metas,data:&[9]},&[acct,dest,auth],signers)
}
// Entitled holder of a store: the Shopkeeper owner; None once the Shopkeeper no longer exists (burned).
fn holder(pid:&Address,asset:&AccountView,team:u16)->Result<Option<Address>,ProgramError>{
    pda(pid,&[b"agent",&team.to_le_bytes(),&[0]],asset)?;
    if !asset.owned_by(&CORE){return Ok(None);}
    let d=asset.try_borrow()?;if d.len()>=33&&d[0]==1{Ok(Some(key(&d,1)?))}else{Ok(None)}
}
fn treasury_owner(c:&[u8;CONFIG_LEN],t:&AccountView)->Result<Address,ProgramError>{
    need(t.address()==&key(c,c::TREASURY)?&&t.owned_by(&pinocchio_token::ID)&&t.data_len()==165,5)?;key(&t.try_borrow()?,32)
}
// Wind-down end state: every store refunded and closed, nothing owed in USDC.
fn wound_down(c:&[u8;CONFIG_LEN])->ProgramResult{need(c[c::WIND]==1&&x16(c,c::CLOSED)?==x16(c,c::MINTED)?&&x8(c,c::LIABILITIES)?==0,13)}

pub fn dispatch(op:u8,pid:&Address,a:&mut[AccountView],d:&[u8])->ProgramResult{
    match op{
        0=>initialize(pid,a,d),1=>set_params(pid,a,d),2=>set_ctown(pid,a),3=>set_collection(pid,a),
        4=>set_role(pid,a,c::OPERATOR),5=>set_role(pid,a,c::PAUSER),6=>set_paused(pid,a,d),
        7=>propose_admin(pid,a),8=>accept_admin(pid,a),9=>sweep_fees(pid,a),10=>set_founding(pid,a,d),
        11=>resolve(pid,a,d),12=>mint_team(pid,a),13=>deposit(pid,a,d),14=>withdraw(pid,a,d),
        15=>set_policy(pid,a,d),16=>lock_bond(pid,a,d),17=>request_unbond(pid,a,d),18=>withdraw_unbond(pid,a),
        19=>dispute(pid,a),20=>settle_sale(pid,a,d),21=>open_order(pid,a,d),22=>deliver(pid,a,d),
        23=>settle_order(pid,a),24=>refund_order(pid,a),25=>rewards(pid,a,d),26=>buyback(pid,a,d),
        27=>refund_store(pid,a),28=>refund_bond(pid,a),29=>close_store(pid,a),30=>drain_usdc(pid,a),
        31=>drain_ctown(pid,a),32=>close_config(pid,a),_=>Err(err(1))
    }
}

#[inline(never)]
fn initialize(pid:&Address,a:&mut[AccountView],d:&[u8])->ProgramResult{
    need(d.len()==130,2)?;
    let admin=acc(a,1)?;sign(admin)?;write(admin)?;
    need(acc(a,3)?.address()==&pinocchio_token::ID&&acc(a,4)?.address()==&SYSTEM,5)?;
    let mint=acc(a,5)?;need(mint.owned_by(&pinocchio_token::ID)&&mint.data_len()==82,5)?;
    let treasury=acc(a,6)?;raw_token(treasury,mint.address())?;
    pda(&LOADER,&[pid.as_ref()],acc(a,10)?)?;
    need(acc(a,10)?.owned_by(&LOADER)&&acc(a,10)?.data_len()>=45,5)?;
    {let pd=acc(a,10)?.try_borrow()?;need(pd[0..4]==[3,0,0,0]&&pd[12]==1&&&pd[13..45]==admin.address().as_ref(),3)?;}
    let cb=pda(pid,&[b"config"],acc(a,0)?)?;vault(pid,acc(a,2)?)?;
    for(i,name)in [(7,b"store_vault".as_slice()),(8,b"pool_vault".as_slice()),(9,b"buyback_vault".as_slice())]{pda(pid,&[name],acc(a,i)?)?;}
    create(pid,acc(a,1)?,acc(a,0)?,b"config",None,CONFIG_LEN,pid)?;
    for(i,name)in [(7,b"store_vault".as_slice()),(8,b"pool_vault".as_slice()),(9,b"buyback_vault".as_slice())]{create_token(pid,acc(a,1)?,acc(a,i)?,name,acc(a,5)?,acc(a,2)?)?;}
    let mut c=[0u8;CONFIG_LEN];c[0]=1;c[1]=1;c[c::BUMP]=cb;
    wk(&mut c,c::ADMIN,admin.address());wk(&mut c,c::OPERATOR,&key(d,0)?);wk(&mut c,c::PAUSER,&key(d,32)?);
    wk(&mut c,c::TREASURY,treasury.address());wk(&mut c,c::USDC,mint.address());
    w8(&mut c,c::MINT_PRICE,5_000_000);for(i,v)in [2500,5000,2500].iter().enumerate(){w16(&mut c,c::SPLIT+i*2,*v)}
    w16(&mut c,c::FEE_SALE,100);w16(&mut c,c::FEE_ORDER,200);w16(&mut c,c::MAX_TEAMS,2000);
    for(i,v)in [3,10,25,50].iter().enumerate(){w16(&mut c,c::CAPACITY+i*2,*v)}
    c[c::URI_LEN]=d[64];need(d[64]<=64&&core::str::from_utf8(&d[65..65+d[64] as usize]).is_ok(),2)?;c[c::URI..c::URI+64].copy_from_slice(&d[65..129]);
    w8(&mut c,c::GLOBAL_CAP,100_000_000);c[c::BURN]=d[129];flag(c[c::BURN])?;
    // safe numbers before set_params: 24 h review, 7 d unbond, 5% reward cap, tiers above Starter out of reach
    c[c::REVIEW..c::REVIEW+4].copy_from_slice(&86400u32.to_le_bytes());c[c::COOLDOWN..c::COOLDOWN+4].copy_from_slice(&604800u32.to_le_bytes());
    w16(&mut c,c::REWARD_CAP,500);for i in 1..4{w8(&mut c,c::THRESH+i*8,u64::MAX);}
    save(&mut a[0],&c)
}
#[inline(never)]
fn set_params(pid:&Address,a:&mut[AccountView],d:&[u8])->ProgramResult{
    need(d.len()==144,2)?;let mut c=cfg(a,pid)?;auth(&c,c::ADMIN,acc(a,1)?)?;
    let price=x8(d,0)?;let split=[x16(d,8)?,x16(d,10)?,x16(d,12)?];
    need(price>0&&split.iter().map(|x|*x as u32).sum::<u32>()==10000,2)?;
    let fs=x16(d,14)?;let fo=x16(d,16)?;let max=x16(d,18)?;
    need(fs<=1000&&fo<=1000&&max>=x16(&c,c::MINTED)?&&max<=2000,2)?;
    let mut prev=0;for i in 0..4{let t=x8(d,20+i*8)?;need(t>=prev,2)?;prev=t;}
    let mut prev_cap=0;for i in 0..4{let n=x16(d,52+i*2)?;need(n>=prev_cap&&n<=100,2)?;prev_cap=n;}
    let cooldown=u32::from_le_bytes(d[60..64].try_into().map_err(|_|err(2))?);
    let review=u32::from_le_bytes(d[64..68].try_into().map_err(|_|err(2))?);
    let rc=x16(d,68)?;let global=x8(d,70)?;let uri_len=d[78] as usize;
    need(cooldown<=30*86400&&review>=60&&review<=30*86400&&rc<=500&&global>0&&uri_len<=64,2)?;
    need(core::str::from_utf8(&d[79..79+uri_len]).is_ok(),2)?;
    flag(d[143])?;
    w8(&mut c,c::MINT_PRICE,price);for i in 0..3{w16(&mut c,c::SPLIT+i*2,split[i]);}
    w16(&mut c,c::FEE_SALE,fs);w16(&mut c,c::FEE_ORDER,fo);w16(&mut c,c::MAX_TEAMS,max);
    c[c::THRESH..c::THRESH+32].copy_from_slice(&d[20..52]);c[c::CAPACITY..c::CAPACITY+8].copy_from_slice(&d[52..60]);
    c[c::COOLDOWN..c::COOLDOWN+4].copy_from_slice(&d[60..64]);c[c::REVIEW..c::REVIEW+4].copy_from_slice(&d[64..68]);
    w16(&mut c,c::REWARD_CAP,rc);w8(&mut c,c::GLOBAL_CAP,global);c[c::URI_LEN]=uri_len as u8;c[c::URI..c::URI+64].copy_from_slice(&d[79..143]);
    c[c::BURN]=d[143];save(&mut a[0],&c)
}

#[inline(never)]
fn set_ctown(pid:&Address,a:&mut[AccountView])->ProgramResult{
    let mut c=cfg(a,pid)?;auth(&c,c::ADMIN,acc(a,1)?)?;need(key(&c,c::CTOWN)?==Address::default(),6)?;
    let mint=acc(a,2)?;let program=acc(a,6)?.address();
    need((program==&pinocchio_token::ID||program==&TOKEN_2022)&&mint.owned_by(program)&&mint.data_len()>=82&&mint.address()!=&key(&c,c::USDC)?,5)?;
    need(mint.try_borrow()?[45]==1,5)?;
    let v=acc(a,3)?;vault(pid,v)?;need(acc(a,7)?.address()==&SYSTEM,5)?;
    let size=ctown_account_size(mint,program)?;
    create_ctown_token(pid,acc(a,1)?,acc(a,4)?,b"bond_vault",mint,v,program,size)?;
    create_ctown_token(pid,acc(a,1)?,acc(a,5)?,b"buyback_out",mint,v,program,size)?;
    wk(&mut c,c::CTOWN,mint.address());wk(&mut c,c::CTOWN_PROGRAM,program);save(&mut a[0],&c)
}
#[inline(never)]
fn set_collection(pid:&Address,a:&mut[AccountView])->ProgramResult{
    let mut c=cfg(a,pid)?;auth(&c,c::ADMIN,acc(a,1)?)?;need(key(&c,c::COLLECTION)?==Address::default(),6)?;
    let col=acc(a,2)?;need(col.owned_by(&CORE)&&col.data_len()>=34,5)?;
    let authority=acc(a,3)?;pda(pid,&[b"collection_authority"],authority)?;
    let d=col.try_borrow()?;need(d[0]==5&&&d[1..33]==authority.address().as_ref(),5)?;
    wk(&mut c,c::COLLECTION,col.address());drop(d);save(&mut a[0],&c)
}
#[inline(never)]
fn set_role(pid:&Address,a:&mut[AccountView],offset:usize)->ProgramResult{
    let mut c=cfg(a,pid)?;auth(&c,c::ADMIN,acc(a,1)?)?;need(acc(a,2)?.address()!=&Address::default(),2)?;
    wk(&mut c,offset,acc(a,2)?.address());save(&mut a[0],&c)
}
#[inline(never)]
fn set_paused(pid:&Address,a:&mut[AccountView],d:&[u8])->ProgramResult{
    // 0 resume, 1 pause, 2 wind-down = pause for good (admin only, no resume after it)
    need(d.len()==1&&d[0]<=2,2)?;let mut c=cfg(a,pid)?;sign(acc(a,1)?)?;
    need(acc(a,1)?.address()==&key(&c,c::ADMIN)?||(d[0]<2&&acc(a,1)?.address()==&key(&c,c::PAUSER)?),3)?;
    need(c[c::WIND]==0||d[0]!=0,13)?;
    c[c::PAUSED]=(d[0]>0) as u8;if d[0]==2{c[c::WIND]=1;}save(&mut a[0],&c)?;emit(17,d);Ok(())
}
#[inline(never)]
fn propose_admin(pid:&Address,a:&mut[AccountView])->ProgramResult{let mut c=cfg(a,pid)?;auth(&c,c::ADMIN,acc(a,1)?)?;wk(&mut c,c::PENDING,acc(a,2)?.address());save(&mut a[0],&c)}
#[inline(never)]
fn accept_admin(pid:&Address,a:&mut[AccountView])->ProgramResult{let mut c=cfg(a,pid)?;auth(&c,c::PENDING,acc(a,1)?)?;wk(&mut c,c::ADMIN,acc(a,1)?.address());wk(&mut c,c::PENDING,&Address::default());save(&mut a[0],&c)}
#[inline(never)]
fn sweep_fees(pid:&Address,a:&mut[AccountView])->ProgramResult{
    let mut c=cfg(a,pid)?;auth(&c,c::ADMIN,acc(a,1)?)?;let n=x8(&c,c::FEES)?;need(n>0,6)?;
    let mint=key(&c,c::USDC)?;let v=acc(a,2)?;let vb=vault(pid,v)?;
    tpda(pid,b"store_vault",acc(a,3)?,&mint,v.address())?;need(acc(a,4)?.address()==&key(&c,c::TREASURY)?,5)?;raw_token(acc(a,4)?,&mint)?;
    let b=[vb];let seeds=bump_signer(b"vault",&b);transfer(acc(a,3)?,acc(a,4)?,v,n,&[Signer::from(&seeds)])?;
    w8(&mut c,c::FEES,0);dec(&mut c,c::LIABILITIES,n)?;
    check_liab(&c,acc(a,3)?)?;save(&mut a[0],&c)
}
#[inline(never)]
fn set_founding(pid:&Address,a:&mut[AccountView],d:&[u8])->ProgramResult{
    need(d.len()==1,2)?;flag(d[0])?;let c=cfg(a,pid)?;auth(&c,c::ADMIN,acc(a,1)?)?;
    let mut st=store(acc(a,2)?,pid)?;st[s::FOUNDING]=d[0];save(&mut a[2],&st)
}

fn core_asset(pid:&Address,a:&[AccountView],team:u16,role:u8,uri_base:&[u8])->ProgramResult{
    let asset=acc(a,10+role as usize)?;let tid=team.to_le_bytes();let roleb=[role];
    let ab=pda(pid,&[b"agent",&tid,&roleb],asset)?;let cb=pda(pid,&[b"collection_authority"],acc(a,9)?)?;
    let label=match role{0=>"Shopkeeper",1=>"Buyer",2=>"Maker",_=>"Host"};
    let name=alloc::format!("CT #{} {}",team,label);let uri=alloc::format!("{}{}",core::str::from_utf8(uri_base).map_err(|_|err(2))?,team as u32*4+role as u32);
    let mut data=Vec::with_capacity(16+name.len()+uri.len());data.push(20);data.push(0);
    data.extend_from_slice(&(name.len() as u32).to_le_bytes());data.extend_from_slice(name.as_bytes());
    data.extend_from_slice(&(uri.len() as u32).to_le_bytes());data.extend_from_slice(uri.as_bytes());data.extend_from_slice(&[0,0]);
    let core=acc(a,14)?;let metas=[
        InstructionAccount::writable_signer(asset.address()),InstructionAccount::writable(acc(a,8)?.address()),
        InstructionAccount::readonly_signer(acc(a,9)?.address()),InstructionAccount::writable_signer(acc(a,1)?.address()),
        InstructionAccount::readonly(acc(a,1)?.address()),InstructionAccount::readonly(core.address()),
        InstructionAccount::readonly(acc(a,15)?.address()),InstructionAccount::readonly(core.address())];
    let abuf=[ab];let cbuf=[cb];
    let sa=[Seed::from(b"agent"),Seed::from(&tid),Seed::from(&roleb),Seed::from(&abuf)];
    let sc=[Seed::from(b"collection_authority"),Seed::from(&cbuf)];
    unsquat(asset,acc(a,1)?,&[Signer::from(&sa)])?;
    let signers=[Signer::from(&sa),Signer::from(&sc)];
    invoke_signed(&InstructionView{program_id:&CORE,accounts:&metas,data:&data},
        &[asset,acc(a,8)?,acc(a,9)?,acc(a,1)?,acc(a,1)?,core,acc(a,15)?,core],&signers)
}
#[inline(never)]
fn mint_team(pid:&Address,a:&mut[AccountView])->ProgramResult{
    let mut c=cfg(a,pid)?;need(c[c::PAUSED]==0,10)?;
    let payer=acc(a,1)?;sign(payer)?;write(payer)?;
    need(acc(a,14)?.address()==&CORE&&acc(a,15)?.address()==&SYSTEM,5)?;
    let mint=key(&c,c::USDC)?;need(acc(a,7)?.address()==&mint,5)?;
    need(acc(a,8)?.address()==&key(&c,c::COLLECTION)?&&acc(a,8)?.owned_by(&CORE),5)?;
    pda(pid,&[b"collection_authority"],acc(a,9)?)?;
    let v=Address::find_program_address(&[b"vault"],pid).0;
    token(acc(a,2)?,&mint,payer.address())?;
    tpda(pid,b"buyback_vault",acc(a,4)?,&mint,&v)?;tpda(pid,b"pool_vault",acc(a,5)?,&mint,&v)?;
    need(acc(a,6)?.address()==&key(&c,c::TREASURY)?,5)?;raw_token(acc(a,6)?,&mint)?;
    let team=x16(&c,c::MINTED)?.checked_add(1).ok_or(err(7))?;need(team<=x16(&c,c::MAX_TEAMS)?,9)?;
    let tid=team.to_le_bytes();let sb=pda(pid,&[b"store",&tid],acc(a,3)?)?;
    let price=x8(&c,c::MINT_PRICE)?;need(price>0,2)?;
    let b=fee(price,x16(&c,c::SPLIT)?)?;let p=fee(price,x16(&c,c::SPLIT+2)?)?;let t=sub(price,add(b,p)?)?;
    transfer(acc(a,2)?,acc(a,4)?,payer,b,&[])?;transfer(acc(a,2)?,acc(a,5)?,payer,p,&[])?;transfer(acc(a,2)?,acc(a,6)?,payer,t,&[])?;
    create(pid,payer,acc(a,3)?,b"store",Some(&tid),STORE_LEN,pid)?;
    let mut st=[0u8;STORE_LEN];st[0]=2;st[1]=1;w16(&mut st,s::TEAM,team);st[s::BUMP]=sb;
    w8(&mut st,s::TRADE,price);w8(&mut st,s::DAILY,mul(price,2)?);
    wi(&mut st,s::CREATED,now()?);wi(&mut st,s::REV_DAY,day(now()?));wi(&mut st,s::SPEND_DAY,day(now()?));
    save(&mut a[3],&st)?;
    let uri=&c[c::URI..c::URI+c[c::URI_LEN] as usize];
    for role in 0..4{core_asset(pid,a,team,role,uri)?;}
    w16(&mut c,c::MINTED,team);
    inc(&mut c,c::TOTALS,price)?;
    inc(&mut c,c::TOTALS+8,p)?;
    save(&mut a[0],&c)?;emit(1,&tid);Ok(())
}
#[inline(never)]
fn deposit(pid:&Address,a:&mut[AccountView],d:&[u8])->ProgramResult{
    need(d.len()==8,2)?;let n=x8(d,0)?;need(n>0,2)?;let mut c=cfg(a,pid)?;need(c[c::PAUSED]==0,10)?;
    let mut st=store(acc(a,3)?,pid)?;let team=x16(&st,s::TEAM)?;
    owner(acc(a,2)?,pid,team,acc(a,1)?)?;
    let mint=key(&c,c::USDC)?;token(acc(a,4)?,&mint,acc(a,1)?.address())?;
    let v=acc(a,6)?;vault(pid,v)?;tpda(pid,b"store_vault",acc(a,5)?,&mint,v.address())?;
    transfer(acc(a,4)?,acc(a,5)?,acc(a,1)?,n,&[])?;
    inc(&mut st,s::BAL,n)?;inc(&mut c,c::LIABILITIES,n)?;
    check_liab(&c,acc(a,5)?)?;save(&mut a[3],&st)?;save(&mut a[0],&c)?;emit(2,d);Ok(())
}
#[inline(never)]
fn withdraw(pid:&Address,a:&mut[AccountView],d:&[u8])->ProgramResult{
    need(d.len()==8,2)?;let n=x8(d,0)?;need(n>0,2)?;let mut c=cfg(a,pid)?;
    let mut st=store(acc(a,3)?,pid)?;owner(acc(a,2)?,pid,x16(&st,s::TEAM)?,acc(a,1)?)?;
    roll_revenue(&mut st,day(now()?))?;
    let withheld=add(x8(&st,s::REV_CUR)?,x8(&st,s::REV_PREV)?)?;
    need(sub(x8(&st,s::BAL)?,n)?>=withheld,9)?;
    let mint=key(&c,c::USDC)?;let v=acc(a,6)?;let bump=vault(pid,v)?;
    tpda(pid,b"store_vault",acc(a,4)?,&mint,v.address())?;token(acc(a,5)?,&mint,acc(a,1)?.address())?;
    let b=[bump];let seeds=bump_signer(b"vault",&b);
    transfer(acc(a,4)?,acc(a,5)?,v,n,&[Signer::from(&seeds)])?;
    dec(&mut st,s::BAL,n)?;dec(&mut c,c::LIABILITIES,n)?;
    check_liab(&c,acc(a,4)?)?;save(&mut a[3],&st)?;save(&mut a[0],&c)?;emit(3,d);Ok(())
}
#[inline(never)]
fn set_policy(pid:&Address,a:&mut[AccountView],d:&[u8])->ProgramResult{
    need(d.len()==25,2)?;let c=cfg(a,pid)?;let mut st=store(acc(a,3)?,pid)?;
    owner(acc(a,2)?,pid,x16(&st,s::TEAM)?,acc(a,1)?)?;
    flag(d[24])?;let t=x8(d,0)?;let daily=x8(d,8)?;let reserve=x8(d,16)?;
    need(t<=daily,2)?;
    w8(&mut st,s::TRADE,t);w8(&mut st,s::DAILY,daily);w8(&mut st,s::RESERVE,reserve);st[s::AGENTS_PAUSED]=d[24];
    let _=c;save(&mut a[3],&st)?;emit(4,d);Ok(())
}
#[inline(never)]
fn lock_bond(pid:&Address,a:&mut[AccountView],d:&[u8])->ProgramResult{
    need(d.len()==8,2)?;let n=x8(d,0)?;need(n>0,2)?;let c=cfg(a,pid)?;need(c[c::PAUSED]==0,10)?;
    let mint=key(&c,c::CTOWN)?;need(mint!=Address::default(),6)?;
    let mut st=store(acc(a,3)?,pid)?;owner(acc(a,2)?,pid,x16(&st,s::TEAM)?,acc(a,1)?)?;
    let decimals=ctown_mint(&c,acc(a,8)?,acc(a,7)?)?;let program=acc(a,7)?.address();
    let v=acc(a,6)?;vault(pid,v)?;pda(pid,&[b"bond_vault"],acc(a,5)?)?;
    let before=ctown_token(acc(a,5)?,&mint,v.address(),program)?;ctown_token(acc(a,4)?,&mint,acc(a,1)?.address(),program)?;
    transfer_checked(acc(a,4)?,acc(a,8)?,acc(a,5)?,acc(a,1)?,program,decimals,n,&[])?;
    // credit what actually arrived: a Token-2022 mint with a transfer fee delivers less than n
    let got=sub(ctown_token(acc(a,5)?,&mint,v.address(),program)?,before)?;need(got>0,9)?;
    inc(&mut st,s::BOND,got)?;save(&mut a[3],&st)?;emit(5,&got.to_le_bytes());Ok(())
}
#[inline(never)]
fn request_unbond(pid:&Address,a:&mut[AccountView],d:&[u8])->ProgramResult{
    need(d.len()==8,2)?;let n=x8(d,0)?;need(n>0,2)?;let c=cfg(a,pid)?;
    let mut st=store(acc(a,3)?,pid)?;owner(acc(a,2)?,pid,x16(&st,s::TEAM)?,acc(a,1)?)?;
    need(x8(&st,s::UNBOND)?==0,9)?;let left=sub(x8(&st,s::BOND)?,n)?;
    need(x16(&st,s::ACTIVE)?<=capacity(&c,left)?,9)?;
    w8(&mut st,s::UNBOND,n);wi(&mut st,s::UNBOND_AT,now()?.checked_add(u32::from_le_bytes(c[c::COOLDOWN..c::COOLDOWN+4].try_into().map_err(|_|err(2))?) as i64).ok_or(err(7))?);
    save(&mut a[3],&st)?;emit(6,d);Ok(())
}
#[inline(never)]
fn withdraw_unbond(pid:&Address,a:&mut[AccountView])->ProgramResult{
    let c=cfg(a,pid)?;let mut st=store(acc(a,3)?,pid)?;
    owner(acc(a,2)?,pid,x16(&st,s::TEAM)?,acc(a,1)?)?;
    let n=x8(&st,s::UNBOND)?;need(n>0&&now()?>=xi(&st,s::UNBOND_AT)?,9)?;
    let mint=key(&c,c::CTOWN)?;let v=acc(a,6)?;let bump=vault(pid,v)?;
    let decimals=ctown_mint(&c,acc(a,8)?,acc(a,7)?)?;let program=acc(a,7)?.address();
    pda(pid,&[b"bond_vault"],acc(a,4)?)?;ctown_token(acc(a,4)?,&mint,v.address(),program)?;ctown_token(acc(a,5)?,&mint,acc(a,1)?.address(),program)?;
    need(x16(&st,s::ACTIVE)?<=capacity(&c,sub(x8(&st,s::BOND)?,n)?)?,9)?;
    let b=[bump];let seeds=bump_signer(b"vault",&b);
    transfer_checked(acc(a,4)?,acc(a,8)?,acc(a,5)?,v,program,decimals,n,&[Signer::from(&seeds)])?;
    dec(&mut st,s::BOND,n)?;w8(&mut st,s::UNBOND,0);wi(&mut st,s::UNBOND_AT,0);
    save(&mut a[3],&st)?;emit(7,&n.to_le_bytes());Ok(())
}
fn asset_owner(pid:&Address,asset:&AccountView,team:u16)->Result<Address,ProgramError>{
    pda(pid,&[b"agent",&team.to_le_bytes(),&[0]],asset)?;need(asset.owned_by(&CORE)&&asset.data_len()>=33,5)?;
    let d=asset.try_borrow()?;need(d[0]==1,5)?;key(&d,1)
}
fn pair(pid:&Address,a:&[AccountView],bi:usize,si:usize,ba:usize,sa:usize)->Result<([u8;STORE_LEN],[u8;STORE_LEN]),ProgramError>{
    let b=store(acc(a,bi)?,pid)?;let s=store(acc(a,si)?,pid)?;
    let bt=x16(&b,s::TEAM)?;let st=x16(&s,s::TEAM)?;need(bt!=st,9)?;
    need(asset_owner(pid,acc(a,ba)?,bt)?!=asset_owner(pid,acc(a,sa)?,st)?,9)?;Ok((b,s))
}
#[inline(never)]
fn settle_sale(pid:&Address,a:&mut[AccountView],d:&[u8])->ProgramResult{
    need(d.len()==16,2)?;let seq=x8(d,0)?;let n=x8(d,8)?;
    let mut c=cfg(a,pid)?;auth(&c,c::OPERATOR,acc(a,1)?)?;
    let (mut buyer,mut seller)=pair(pid,a,2,3,4,5)?;
    let day=day(now()?);spend(&mut c,&mut buyer,n,seq,day)?;
    let f=fee(n,x16(&c,c::FEE_SALE)?)?;credit(&mut seller,sub(n,f)?,day)?;
    inc(&mut c,c::FEES,f)?;
    counter(&mut buyer,1,1)?;counter(&mut buyer,3,n)?;counter(&mut buyer,4,f)?;
    counter(&mut seller,0,1)?;counter(&mut seller,2,sub(n,f)?)?;
    let mint=key(&c,c::USDC)?;let v=Address::find_program_address(&[b"vault"],pid).0;
    tpda(pid,b"store_vault",acc(a,6)?,&mint,&v)?;check_liab(&c,acc(a,6)?)?;
    save(&mut a[2],&buyer)?;save(&mut a[3],&seller)?;save(&mut a[0],&c)?;emit(8,d);Ok(())
}
#[inline(never)]
fn open_order(pid:&Address,a:&mut[AccountView],d:&[u8])->ProgramResult{
    need(d.len()==32,2)?;let id=x8(d,0)?;let seq=x8(d,8)?;let n=x8(d,16)?;let deadline=xi(d,24)?;
    let t=now()?;need(deadline>t&&deadline<=t+30*86400,2)?;
    let mut c=cfg(a,pid)?;auth(&c,c::OPERATOR,acc(a,1)?)?;write(acc(a,1)?)?;
    let (mut buyer,mut seller)=pair(pid,a,2,3,4,5)?;
    need(x16(&buyer,s::ACTIVE)?<capacity(&c,x8(&buyer,s::BOND)?)?&&
         x16(&seller,s::ACTIVE)?<capacity(&c,x8(&seller,s::BOND)?)?,9)?;
    spend(&mut c,&mut buyer,n,seq,day(t))?;
    inc(&mut buyer,s::ESCROW,n)?;
    inc16(&mut buyer,s::ACTIVE)?;
    inc16(&mut seller,s::ACTIVE)?;
    need(acc(a,8)?.address()==&SYSTEM,5)?;
    let idb=id.to_le_bytes();pda(pid,&[b"order",&idb],acc(a,6)?)?;
    create(pid,acc(a,1)?,acc(a,6)?,b"order",Some(&idb),ORDER_LEN,pid)?;
    let mut ord=[0u8;ORDER_LEN];ord[0]=3;ord[1]=1;w8(&mut ord,o::ID,id);
    w16(&mut ord,o::BUYER,x16(&buyer,s::TEAM)?);w16(&mut ord,o::SELLER,x16(&seller,s::TEAM)?);
    w8(&mut ord,o::AMOUNT,n);w16(&mut ord,o::FEE,x16(&c,c::FEE_ORDER)?);
    wi(&mut ord,o::CREATED,t);wi(&mut ord,o::DEADLINE,deadline);ord[o::STATUS]=1;wk(&mut ord,o::PAYER,acc(a,1)?.address());
    let mint=key(&c,c::USDC)?;let v=Address::find_program_address(&[b"vault"],pid).0;
    tpda(pid,b"store_vault",acc(a,7)?,&mint,&v)?;check_liab(&c,acc(a,7)?)?;
    save(&mut a[6],&ord)?;save(&mut a[2],&buyer)?;save(&mut a[3],&seller)?;save(&mut a[0],&c)?;emit(9,d);Ok(())
}
#[inline(never)]
fn deliver(pid:&Address,a:&mut[AccountView],d:&[u8])->ProgramResult{
    need(d.len()==32,2)?;let c=cfg(a,pid)?;auth(&c,c::OPERATOR,acc(a,1)?)?;need(c[c::PAUSED]==0,10)?;
    let mut ord=order(acc(a,2)?,pid)?;let t=now()?;
    need(ord[o::STATUS]==1&&t<=xi(&ord,o::DEADLINE)?,11)?;
    ord[o::HASH..o::HASH+32].copy_from_slice(d);ord[o::STATUS]=2;wi(&mut ord,o::DELIVERED,t);
    let window=u32::from_le_bytes(c[c::REVIEW..c::REVIEW+4].try_into().map_err(|_|err(2))?) as i64;
    wi(&mut ord,o::REVIEW,t.checked_add(window).ok_or(err(7))?);
    save(&mut a[2],&ord)?;emit(10,d);Ok(())
}
#[inline(never)]
fn dispute(pid:&Address,a:&mut[AccountView])->ProgramResult{
    let _c=cfg(a,pid)?;let mut st=store(acc(a,3)?,pid)?;
    owner(acc(a,2)?,pid,x16(&st,s::TEAM)?,acc(a,1)?)?;
    let mut ord=order(acc(a,4)?,pid)?;need(x16(&ord,o::BUYER)?==x16(&st,s::TEAM)?,5)?;
    let limit=if ord[o::STATUS]==1{xi(&ord,o::DEADLINE)?}else{xi(&ord,o::REVIEW)?};
    need((ord[o::STATUS]==1||ord[o::STATUS]==2)&&now()?<limit,11)?;
    ord[o::STATUS]=3;counter(&mut st,6,1)?;save(&mut a[3],&st)?;save(&mut a[4],&ord)?;emit(13,&ord[o::ID..o::ID+8]);Ok(())
}
#[inline(never)]
fn final_order(pid:&Address,a:&mut[AccountView],mode:u8)->ProgramResult{
    let mut c=cfg(a,pid)?;let ord=order(acc(a,1)?,pid)?;
    let t=now()?;match mode{0=>need(ord[o::STATUS]==2&&t>=xi(&ord,o::REVIEW)?,11)?,
        1=>need(ord[o::STATUS]==1&&t>xi(&ord,o::DEADLINE)?,11)?,
        _=>need(ord[o::STATUS]==3,11)?}
    let mut buyer=store(acc(a,2)?,pid)?;let mut seller=store(acc(a,3)?,pid)?;
    need(x16(&buyer,s::TEAM)?==x16(&ord,o::BUYER)?&&x16(&seller,s::TEAM)?==x16(&ord,o::SELLER)?,5)?;
    let n=x8(&ord,o::AMOUNT)?;dec(&mut buyer,s::ESCROW,n)?;
    dec16(&mut buyer,s::ACTIVE)?;
    dec16(&mut seller,s::ACTIVE)?;
    if mode==1||mode==3{
        inc(&mut buyer,s::BAL,n)?;counter(&mut buyer,7,1)?;
    }else{
        let f=fee(n,x16(&ord,o::FEE)?)?;credit(&mut seller,sub(n,f)?,day(t))?;
        inc(&mut c,c::FEES,f)?;
        counter(&mut buyer,1,1)?;counter(&mut buyer,3,n)?;counter(&mut buyer,4,f)?;
        counter(&mut seller,0,1)?;counter(&mut seller,2,sub(n,f)?)?;
    }
    need(acc(a,4)?.address()==&key(&ord,o::PAYER)?,5)?;
    let mint=key(&c,c::USDC)?;let v=Address::find_program_address(&[b"vault"],pid).0;
    tpda(pid,b"store_vault",acc(a,5)?,&mint,&v)?;check_liab(&c,acc(a,5)?)?;
    save(&mut a[2],&buyer)?;save(&mut a[3],&seller)?;save(&mut a[0],&c)?;
    let (left,right)=a.split_at_mut(4);close_order(&mut left[1],&mut right[0])?;
    emit(if mode==1{12}else if mode==0{11}else{14},&ord[o::ID..o::ID+8]);Ok(())
}
#[inline(never)]
fn settle_order(pid:&Address,a:&mut[AccountView])->ProgramResult{final_order(pid,a,0)}
#[inline(never)]
fn refund_order(pid:&Address,a:&mut[AccountView])->ProgramResult{final_order(pid,a,1)}
#[inline(never)]
fn resolve(pid:&Address,a:&mut[AccountView],d:&[u8])->ProgramResult{
    need(d.len()==1,2)?;flag(d[0])?;let c=cfg(a,pid)?;auth(&c,c::ADMIN,acc(a,1)?)?;
    // disputed: admin rules either way; undelivered during wind-down: refund to the buyer only
    let ord=order(acc(a,2)?,pid)?;need(ord[o::STATUS]==3||(ord[o::STATUS]==1&&c[c::WIND]==1&&d[0]==1),11)?;
    // Reuse finalization with the shared account order expected by permissionless settlement.
    let shifted=&mut a[1..];
    // shifted[0] is admin, so resolve executes its own accounting below.
    let _=shifted;resolve_accounts(pid,a,d[0])
}
#[inline(never)]
fn resolve_accounts(pid:&Address,a:&mut[AccountView],refund:u8)->ProgramResult{
    let mut c=cfg(a,pid)?;let ord=order(acc(a,2)?,pid)?;
    let mut buyer=store(acc(a,3)?,pid)?;let mut seller=store(acc(a,4)?,pid)?;
    need(x16(&buyer,s::TEAM)?==x16(&ord,o::BUYER)?&&x16(&seller,s::TEAM)?==x16(&ord,o::SELLER)?,5)?;
    let n=x8(&ord,o::AMOUNT)?;dec(&mut buyer,s::ESCROW,n)?;
    dec16(&mut buyer,s::ACTIVE)?;
    dec16(&mut seller,s::ACTIVE)?;
    if refund==1{inc(&mut buyer,s::BAL,n)?;counter(&mut buyer,7,1)?;}
    else{let f=fee(n,x16(&ord,o::FEE)?)?;credit(&mut seller,sub(n,f)?,day(now()?))?;inc(&mut c,c::FEES,f)?;counter(&mut seller,0,1)?;counter(&mut seller,2,sub(n,f)?)?;}
    need(acc(a,5)?.address()==&key(&ord,o::PAYER)?,5)?;
    let mint=key(&c,c::USDC)?;let v=Address::find_program_address(&[b"vault"],pid).0;
    tpda(pid,b"store_vault",acc(a,6)?,&mint,&v)?;check_liab(&c,acc(a,6)?)?;
    save(&mut a[3],&buyer)?;save(&mut a[4],&seller)?;save(&mut a[0],&c)?;
    let (left,right)=a.split_at_mut(5);close_order(&mut left[2],&mut right[0])?;
    emit(14,&ord[o::ID..o::ID+8]);Ok(())
}
#[inline(never)]
fn rewards(pid:&Address,a:&mut[AccountView],d:&[u8])->ProgramResult{
    need(d.len()>=10,2)?;let wanted=xi(d,0)?;let count=x16(d,8)? as usize;
    need(count>0&&count<=25&&d.len()==10+count*8&&a.len()==6+count&&acc(a,5)?.address()==&pinocchio_token::ID,2)?;
    let mut c=cfg(a,pid)?;auth(&c,c::OPERATOR,acc(a,1)?)?;need(c[c::PAUSED]==0,10)?;
    let today=day(now()?);need(wanted==today,2)?;
    let mint=key(&c,c::USDC)?;let v=acc(a,4)?;let bump=vault(pid,v)?;
    let before=tpda(pid,b"pool_vault",acc(a,2)?,&mint,v.address())?;
    tpda(pid,b"store_vault",acc(a,3)?,&mint,v.address())?;
    if xi(&c,c::REWARD_DAY)?!=today{wi(&mut c,c::REWARD_DAY,today);w8(&mut c,c::REWARD_PAID,0);w8(&mut c,c::REWARD_BASE,before);}
    let mut states=Vec::with_capacity(count);let mut total=0u64;
    for i in 0..count{
        for j in 0..i{need(acc(a,6+i)?.address()!=acc(a,6+j)?.address(),5)?;}
        let mut st=store(acc(a,6+i)?,pid)?;need(st[s::FOUNDING]==0,9)?;
        let n=x8(d,10+i*8)?;need(n>0,2)?;total=add(total,n)?;
        inc(&mut st,s::BAL,n)?;counter(&mut st,5,n)?;states.push(st);
    }
    let cap=fee(x8(&c,c::REWARD_BASE)?,x16(&c,c::REWARD_CAP)?)?;
    need(total<=before&&add(x8(&c,c::REWARD_PAID)?,total)?<=cap,9)?;
    let b=[bump];let seeds=bump_signer(b"vault",&b);
    transfer(acc(a,2)?,acc(a,3)?,v,total,&[Signer::from(&seeds)])?;
    inc(&mut c,c::REWARD_PAID,total)?;
    inc(&mut c,c::LIABILITIES,total)?;
    inc(&mut c,c::TOTALS+16,total)?;
    check_liab(&c,acc(a,3)?)?;
    for(i,st)in states.iter().enumerate(){save(&mut a[6+i],st)?;let mut ev=[0u8;10];ev[..2].copy_from_slice(&st[s::TEAM..s::TEAM+2]);ev[2..].copy_from_slice(&d[10+i*8..18+i*8]);emit(15,&ev);}
    save(&mut a[0],&c)
}
#[inline(never)]
fn buyback(pid:&Address,a:&mut[AccountView],d:&[u8])->ProgramResult{
    need(d.len()>=18,2)?;let n=x8(d,0)?;let min=x8(d,8)?;let len=x16(d,16)? as usize;
    need(n>0&&min>0&&d.len()==18+len&&len>8&&a.len()>=8&&a.len()<=71,2)?;
    let mut c=cfg(a,pid)?;auth(&c,c::OPERATOR,acc(a,1)?)?;need(c[c::PAUSED]==0,10)?;
    let v=acc(a,2)?;let bump=vault(pid,v)?;let usdc=key(&c,c::USDC)?;let ctown=key(&c,c::CTOWN)?;
    need(ctown!=Address::default()&&acc(a,5)?.address()==&JUPITER,5)?;
    let decimals=ctown_mint(&c,acc(a,4)?,acc(a,7)?)?;let program=acc(a,7)?.address();
    let before_in=tpda(pid,b"buyback_vault",acc(a,3)?,&usdc,v.address())?;
    pda(pid,&[b"buyback_out"],acc(a,6)?)?;let before_out=ctown_token(acc(a,6)?,&ctown,v.address(),program)?;
    need(before_in>=n,9)?;
    let mut metas=Vec::with_capacity(a.len()-8);let mut views=Vec::with_capacity(a.len()-8);
    let mut has_in=false;let mut has_out=false;let mut has_v=false;
    for ai in &a[8..]{
        if ai.address()==acc(a,3)?.address(){has_in=true;need(ai.is_writable(),5)?;}
        if ai.address()==acc(a,6)?.address(){has_out=true;need(ai.is_writable(),5)?;}
        if ai.address()==v.address(){has_v=true;}
        if (ai.owned_by(&pinocchio_token::ID)||ai.owned_by(&TOKEN_2022))&&ai.data_len()>=165{
            let data=ai.try_borrow()?;
            if &data[32..64]==v.address().as_ref(){need(ai.address()==acc(a,3)?.address()||ai.address()==acc(a,6)?.address(),5)?;}
        }
        need(!ai.owned_by(pid)||!ai.is_writable(),5)?;
        metas.push(InstructionAccount::new(ai.address(),ai.is_writable(),ai.is_signer()||ai.address()==v.address()));views.push(ai);
    }
    need(has_in&&has_out&&has_v,5)?;
    let b=[bump];let seeds=bump_signer(b"vault",&b);
    invoke_signed_with_slice(&InstructionView{program_id:&JUPITER,accounts:&metas,data:&d[18..]},&views,&[Signer::from(&seeds)])?;
    let spent=sub(before_in,token(acc(a,3)?,&usdc,v.address())?)?;
    let received=sub(ctown_token(acc(a,6)?,&ctown,v.address(),program)?,before_out)?;
    need(spent>0&&spent<=n&&received>=min,12)?;
    // the operator key's whole daily reach, buybacks included, stays under the town cap (a leaked key or a bad route
    // cannot empty the buyback vault in one go)
    let today=day(now()?);if xi(&c,c::GLOBAL_DAY)?!=today{wi(&mut c,c::GLOBAL_DAY,today);w8(&mut c,c::GLOBAL_SPENT,0);}
    need(add(x8(&c,c::GLOBAL_SPENT)?,spent)?<=x8(&c,c::GLOBAL_CAP)?,9)?;inc(&mut c,c::GLOBAL_SPENT,spent)?;
    if c[c::BURN]==1{burn(acc(a,6)?,acc(a,4)?,v,program,decimals,received,&[Signer::from(&seeds)])?;}
    inc(&mut c,c::TOTALS+24,spent)?;
    inc(&mut c,c::TOTALS+32,received)?;
    if c[c::BURN]==1{inc(&mut c,c::TOTALS+40,received)?;}
    save(&mut a[0],&c)?;let mut ev=[0u8;16];ev[..8].copy_from_slice(&spent.to_le_bytes());ev[8..].copy_from_slice(&received.to_le_bytes());emit(16,&ev);Ok(())
}

// ---- v1.1 recovery, wind-down only (a plain pause stays reversible and moves nobody's money) ----
#[inline(never)]
fn refund_store(pid:&Address,a:&mut[AccountView])->ProgramResult{
    let mut c=cfg(a,pid)?;auth(&c,c::ADMIN,acc(a,1)?)?;need(c[c::WIND]==1,13)?;
    let mut st=store(acc(a,2)?,pid)?;let team=x16(&st,s::TEAM)?;let n=x8(&st,s::BAL)?;need(n>0,6)?;
    let mint=key(&c,c::USDC)?;
    match holder(pid,acc(a,3)?,team)?{
        Some(h)=>{token(acc(a,4)?,&mint,&h)?;}
        None=>{need(acc(a,4)?.address()==&key(&c,c::TREASURY)?,5)?;raw_token(acc(a,4)?,&mint)?;}
    }
    let v=acc(a,6)?;let bump=vault(pid,v)?;tpda(pid,b"store_vault",acc(a,5)?,&mint,v.address())?;
    need(acc(a,4)?.address()!=acc(a,5)?.address(),5)?;
    let b=[bump];let seeds=bump_signer(b"vault",&b);transfer(acc(a,5)?,acc(a,4)?,v,n,&[Signer::from(&seeds)])?;
    w8(&mut st,s::BAL,0);w8(&mut st,s::REV_CUR,0);w8(&mut st,s::REV_PREV,0);dec(&mut c,c::LIABILITIES,n)?;
    check_liab(&c,acc(a,5)?)?;save(&mut a[2],&st)?;save(&mut a[0],&c)?;
    let mut ev=[0u8;10];ev[..2].copy_from_slice(&team.to_le_bytes());ev[2..].copy_from_slice(&n.to_le_bytes());emit(18,&ev);Ok(())
}
#[inline(never)]
fn refund_bond(pid:&Address,a:&mut[AccountView])->ProgramResult{
    let c=cfg(a,pid)?;auth(&c,c::ADMIN,acc(a,1)?)?;need(c[c::WIND]==1,13)?;
    let mut st=store(acc(a,2)?,pid)?;let team=x16(&st,s::TEAM)?;let n=x8(&st,s::BOND)?;need(n>0,6)?;
    let mint=key(&c,c::CTOWN)?;let decimals=ctown_mint(&c,acc(a,8)?,acc(a,7)?)?;let program=acc(a,7)?.address();
    let to=match holder(pid,acc(a,3)?,team)?{Some(h)=>h,None=>treasury_owner(&c,acc(a,9)?)?};
    let v=acc(a,6)?;let bump=vault(pid,v)?;pda(pid,&[b"bond_vault"],acc(a,4)?)?;
    ctown_token(acc(a,4)?,&mint,v.address(),program)?;ctown_token(acc(a,5)?,&mint,&to,program)?;
    let b=[bump];let seeds=bump_signer(b"vault",&b);
    transfer_checked(acc(a,4)?,acc(a,8)?,acc(a,5)?,v,program,decimals,n,&[Signer::from(&seeds)])?;
    w8(&mut st,s::BOND,0);w8(&mut st,s::UNBOND,0);wi(&mut st,s::UNBOND_AT,0);save(&mut a[2],&st)?;
    let mut ev=[0u8;10];ev[..2].copy_from_slice(&team.to_le_bytes());ev[2..].copy_from_slice(&n.to_le_bytes());emit(19,&ev);Ok(())
}
#[inline(never)]
fn close_store(pid:&Address,a:&mut[AccountView])->ProgramResult{
    let mut c=cfg(a,pid)?;auth(&c,c::ADMIN,acc(a,1)?)?;need(c[c::WIND]==1,13)?;
    let st=store(acc(a,2)?,pid)?;let team=x16(&st,s::TEAM)?;
    need(x8(&st,s::BAL)?==0&&x8(&st,s::ESCROW)?==0&&x16(&st,s::ACTIVE)?==0&&x8(&st,s::BOND)?==0,9)?;
    // rent to the Shopkeeper holder. To the admin only when the holder is gone or parked on an address that cannot take
    // lamports (a program, a sysvar, a runtime-reserved id, or the store itself), so one holder cannot block the wind-down. Judged by the
    // account itself, never by the writable flag the caller chose.
    let admin=key(&c,c::ADMIN)?;
    match holder(pid,acc(a,3)?,team)?{
        None=>need(acc(a,4)?.address()==&admin,5)?,
        Some(h)=>if acc(a,4)?.address()!=&h{
            let hv=acc(a,5)?;
            need(acc(a,4)?.address()==&admin&&hv.address()==&h&&(hv.executable()||hv.owned_by(&SYSVAR)||RESERVED.contains(&h)||&h==acc(a,2)?.address()),5)?;
        }
    }
    inc16(&mut c,c::CLOSED)?;save(&mut a[0],&c)?;
    let (l,r)=a.split_at_mut(4);close_order(&mut l[2],&mut r[0])?;emit(20,&team.to_le_bytes());Ok(())
}
#[inline(never)]
fn drain_usdc(pid:&Address,a:&mut[AccountView])->ProgramResult{
    let c=cfg(a,pid)?;auth(&c,c::ADMIN,acc(a,1)?)?;wound_down(&c)?;
    let mint=key(&c,c::USDC)?;let v=acc(a,2)?;let bump=vault(pid,v)?;
    need(acc(a,6)?.address()==&key(&c,c::TREASURY)?,5)?;raw_token(acc(a,6)?,&mint)?;
    let b=[bump];let seeds=bump_signer(b"vault",&b);let mut total=0u64;
    for(i,name)in [(3,b"store_vault".as_slice()),(4,b"pool_vault".as_slice()),(5,b"buyback_vault".as_slice())]{
        let n=tpda(pid,name,acc(a,i)?,&mint,v.address())?;
        if n>0{transfer(acc(a,i)?,acc(a,6)?,v,n,&[Signer::from(&seeds)])?;total=add(total,n)?;}
    }
    emit(21,&total.to_le_bytes());Ok(())
}
#[inline(never)]
fn drain_ctown(pid:&Address,a:&mut[AccountView])->ProgramResult{
    let c=cfg(a,pid)?;auth(&c,c::ADMIN,acc(a,1)?)?;wound_down(&c)?;
    let mint=key(&c,c::CTOWN)?;let decimals=ctown_mint(&c,acc(a,8)?,acc(a,7)?)?;let program=acc(a,7)?.address();
    let v=acc(a,2)?;let bump=vault(pid,v)?;let to=treasury_owner(&c,acc(a,6)?)?;ctown_token(acc(a,5)?,&mint,&to,program)?;
    let b=[bump];let seeds=bump_signer(b"vault",&b);let mut total=0u64;
    for(i,name)in [(3,b"bond_vault".as_slice()),(4,b"buyback_out".as_slice())]{
        pda(pid,&[name],acc(a,i)?)?;let n=ctown_token(acc(a,i)?,&mint,v.address(),program)?;
        if n>0{transfer_checked(acc(a,i)?,acc(a,8)?,acc(a,5)?,v,program,decimals,n,&[Signer::from(&seeds)])?;total=add(total,n)?;}
    }
    emit(21,&total.to_le_bytes());Ok(())
}
#[inline(never)]
fn close_config(pid:&Address,a:&mut[AccountView])->ProgramResult{
    let c=cfg(a,pid)?;auth(&c,c::ADMIN,acc(a,1)?)?;write(acc(a,1)?)?;wound_down(&c)?;
    need(acc(a,6)?.address()==&pinocchio_token::ID,5)?;
    let v=acc(a,2)?;let bump=vault(pid,v)?;let b=[bump];let seeds=bump_signer(b"vault",&b);
    let usdc=key(&c,c::USDC)?;
    // the token program refuses to close a vault that still holds tokens: drain first
    for(i,name)in [(3,b"store_vault".as_slice()),(4,b"pool_vault".as_slice()),(5,b"buyback_vault".as_slice())]{
        tpda(pid,name,acc(a,i)?,&usdc,v.address())?;close_token(acc(a,i)?,acc(a,1)?,v,&pinocchio_token::ID,&[Signer::from(&seeds)])?;
    }
    let ctown=key(&c,c::CTOWN)?;
    if ctown!=Address::default(){
        let program=acc(a,9)?.address();need(program==&key(&c,c::CTOWN_PROGRAM)?,5)?;
        for(i,name)in [(7,b"bond_vault".as_slice()),(8,b"buyback_out".as_slice())]{
            pda(pid,&[name],acc(a,i)?)?;ctown_token(acc(a,i)?,&ctown,v.address(),program)?;
            close_token(acc(a,i)?,acc(a,1)?,v,program,&[Signer::from(&seeds)])?;
        }
    }
    let (l,r)=a.split_at_mut(1);close_order(&mut l[0],&mut r[0])?;emit(22,&[]);Ok(())
}
