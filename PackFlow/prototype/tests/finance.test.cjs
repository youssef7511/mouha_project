const fs=require('node:fs'),vm=require('node:vm'),assert=require('node:assert/strict');
const elements=new Map(),storage=new Map();
const el=id=>{if(!elements.has(id))elements.set(id,{innerHTML:'',textContent:'',value:'',classList:{add(){},remove(){}},addEventListener(){},showModal(){},close(){},focus(){},setSelectionRange(){}});return elements.get(id)};
class FormDataStub extends Map{constructor(target){super(Object.entries(target.values))}}
const ctx={console,structuredClone,FormData:FormDataStub,localStorage:{getItem:k=>storage.get(k)||null,setItem:(k,v)=>storage.set(k,v)},location:{hash:''},document:{getElementById:el,title:''},window:{addEventListener(){}},setTimeout(){},clearTimeout(){},Blob,URL};vm.createContext(ctx);
for(const file of ['app.js','finance.js'])vm.runInContext(fs.readFileSync('dist/'+file,'utf8'),ctx);
const run=code=>vm.runInContext(code,ctx);
const event=values=>({preventDefault(){},target:{values}});
function call(fn,values,id){ctx.testEvent=event(values);run(`${fn}(testEvent${id?`, '${id}'`:''})`)}
for(const page of ['dashboard','projects','new','clients','stock','reports','payments','costs','workers']){run(`page='${page}';render()`);const html=el('app').innerHTML;assert(!html.includes('undefined'),page);for(const match of html.matchAll(/(?:onclick|oninput|onchange|onsubmit)="([^"]*)"/g)){new Function('event',match[1].replace(/&#39;/g,"'").replace(/&quot;/g,'"').replace(/&amp;/g,'&').replace(/&lt;/g,'<').replace(/&gt;/g,'>'))}}
console.log('All nine screens render; event handlers compile.');
for(const [served,fulfillment] of [[0,'Not served'],[100,'Partially served'],[600,'Fully served']])for(const [paid,status] of [[0,'Unpaid'],[50000,'Partially paid'],[100000,'Fully paid']]){run(`projects[0].served=${served};projects[0].states[6]=${served===600?2:served?1:0};finance.orders['PF-1048']={base:100000,outside:false,charge:0,supplier:0,receipts:[{amount:${paid}}]}`);assert.equal(run('serviceState(projects[0])'),fulfillment);assert.equal(run('paymentState(orderTotal(order(projects[0])),received(order(projects[0])))'),status)}
assert.equal(run('paymentState(null,500)'),'Not priced');assert.equal(run('paymentState(0,0)'),'No payment due');assert.equal(run('paymentState(100,150)'),'Overpaid');assert.equal(run('cents("12.34")'),1234);
console.log('All nine fulfillment/payment combinations, zero/unpriced orders, and credits pass.');
call('saveOrder',{served:'100',base:'1000',outside:'on',service:'Printing',charge:'120',supplier:'80'},'PF-1048');assert.equal(run('orderTotal(order(projects[0]))'),112000);assert.equal(run("allCosts().filter(c=>c.id==='outside-PF-1048').length"),1);assert.equal(run("allCosts().find(c=>c.id==='outside-PF-1048').rate"),8000);assert.equal(run('projects[0].states[6]'),1);
call('saveOrder',{served:'100',base:'1000',service:'Printing',charge:'120',supplier:'80'},'PF-1048');assert.equal(run('orderTotal(order(projects[0]))'),100000);assert.equal(run("allCosts().filter(c=>c.id==='outside-PF-1048').length"),0);
call('recordReceipt',{project:'PF-1048',kind:'refund',amount:'1500',date:'2026-10-11',note:'Invalid'});assert(el('form-error').textContent.includes('cannot exceed'));assert.equal(run('received(order(projects[0]))'),100000);
call('recordReceipt',{project:'PF-1048',kind:'refund',amount:'250',date:'2026-10-11',note:'Partial refund'});assert.equal(run('received(order(projects[0]))'),75000);assert.equal(run('serviceState(projects[0])'),'Partially served');
call('saveServed',{served:'600'},'PF-1048');assert.equal(run('serviceState(projects[0])'),'Fully served');assert.equal(run('received(order(projects[0]))'),75000);
console.log('Outside services reconcile once; refunds and served updates preserve independent status.');
const before=run('stock[0].qty');call('addCost',{project:'PF-1048',category:'Boxes',stock:'BX-001',qty:'10',rate:'0.65',date:'2026-10-11'});assert.equal(run('stock[0].qty'),before-10);assert.equal(run('lineTotal(finance.costs.at(-1))'),650);call('addCost',{project:'PF-1048',category:'Boxes',stock:'BX-001',qty:'999999',rate:'0.65',date:'2026-10-11'});assert.equal(run('stock[0].qty'),before-10);
const oldSalary=run("payroll(finance.workers[0],'2026-10').base");call('saveWorker',{name:'Jamie Davis',role:'Lead',salary:'3000',hours:'160',active:'on'},'W-001');assert.equal(run("payroll(finance.workers[0],'2026-10').base"),oldSalary);assert.equal(run("payroll(finance.workers[0],'2026-11').base"),300000);
run("workerMonth='2026-11'");call('paySalary',{amount:'1000',date:'2026-11-01',note:'Advance'},'W-001');assert.equal(run("received(payroll(finance.workers[0]))"),100000);assert.equal(run("received(payroll(finance.workers[0],'2026-10'))"),140000);assert(storage.has('packflow-finance-v1'));
run("projects.push({id:'PF-2000',boxes:25,states:[0,0,0,0,0,0,0]})");assert.equal(run('orderTotal(order(projects.at(-1)))'),null);
console.log('Stock allocation, overspend guard, salary history, monthly advances, persistence, and new-order defaults pass.');
