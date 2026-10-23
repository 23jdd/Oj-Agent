//! Create an isolated local UI review profile; never writes the user's normal data directory.
use oj_agent::{
    chat::{self, Message},
    session::{SessionStore, SqliteStore},
};
use serde_json::json;
fn main() -> anyhow::Result<()> {
    let large = std::env::args().any(|arg| arg == "--large");
    let directory = std::path::PathBuf::from(if large {
        "target/animation-scroll-review/profile"
    } else {
        "target/chat-review/profile"
    });
    std::fs::create_dir_all(&directory)?;
    let source = if large {
        "function solve(input, emit) { const nodes=Array.from({length:64},(_,i)=>({id:String(i),label:'Readable node '+i,value:i})); const edges=nodes.slice(1).map((n,i)=>({from:String(i),to:n.id})); emit({nodes,edges,explanation:'Full explanation\\n'.repeat(100)+'LAST LINE / 最后一行',state:'完整状态信息'}); return [...input].sort((a,b)=>a-b); }"
    } else {
        "function solve(input, emit) {const a=[...input];emit({values:a,explanation:'读取输入'});a.sort((x,y)=>x-y);emit({values:a,explanation:'排序完成'});return a;}"
    };
    let request: oj_sandbox::Request = serde_json::from_value(json!({"source":source,"cases":[
        {"name":"一般输入","category":"normal","input":[3,1,2],"expected":[1,2,3]},
        {"name":"空输入","category":"boundary","input":[],"expected":[]},
        {"name":"重复与负数","category":"adversarial","input":[2,-1,2],"expected":[-1,2,2]}
    ]}))?;
    let report = oj_sandbox::verify(&request, Default::default()).map_err(anyhow::Error::msg)?;
    anyhow::ensure!(
        report.passed,
        "Review verification failed: {:?}",
        report
            .cases
            .iter()
            .filter(|case| !case.passed)
            .map(|case| (&case.name, &case.error))
            .collect::<Vec<_>>()
    );
    let mut store = SqliteStore::open(&directory.join("sessions.sqlite3"))?;
    let session = store.create_chat("对话界面预览")?;
    let verification = store.add_artifact(
        &session,
        "verification",
        &json!({"request":request,"report":report}),
        &[],
        true,
    )?;
    let bundle = chat::animation::compile_verified(
        &report,
        &verification,
        "本机 JavaScript 示例（界面预览数据）",
        "排序执行过程",
        if large { "graph" } else { "array" },
        &Default::default(),
    )?;
    let artifact = store.add_artifact(
        &session,
        "chat-animation",
        &json!(bundle),
        &[verification],
        true,
    )?;
    store.append_message(
        &session,
        &Message::new(
            "user",
            "解释这段代码，检查边界情况，必要时给我一个动画。".into(),
            vec![],
        ),
    )?;
    store.append_message(&session,&Message::new("assistant",format!("## 执行过程\n\n这段代码对数组进行**升序排序**，输入保持不变。\n\n```javascript\nconst result = [...input].sort((a, b) => a - b);\n```\n\n| 情况 | 结果 |\n| --- | --- |\n| 一般输入 | 通过 |\n| 空输入 | 通过 |\n| 重复与负数 | 通过 |\n\n> 以下是本机实际执行后生成的预览数据，不是对所有输入的正确性证明。\n\n[打开动画：排序执行过程](oj-animation:{artifact})"),vec![]))?;
    println!("Profile: {}", directory.canonicalize()?.display());
    Ok(())
}
