// Evaluator only. Counter text is parsed in Node, outside the selected renderer.
export function leadingNumber(html) {
  const match = html.replace(/^<[^>]*>/, '').match(/^\s*(-?\d+(?:\.\d+)?)(?:\s*<|\s*$)/);
  if (!match) throw Error('Displayed counter has unsupported leading text');
  const value = Number(match[1]);
  if (!Number.isFinite(value)) throw Error('Nonfinite displayed counter');
  return value;
}

export async function createOracle(context, page, selectors) {
  const session = await context.newCDPSession(page);
  const document = await session.send('DOM.getDocument', {depth: 1});
  const nodes = {};
  for (const [key, selector] of Object.entries(selectors)) {
    const {nodeId} = await session.send('DOM.querySelector', {nodeId: document.root.nodeId, selector});
    if (!nodeId) throw Error('Counter element absent: ' + selector);
    nodes[key] = nodeId;
  }
  return async () => {
    const before = Date.now();
    // Generic geometry only: no game variables or counter values are evaluated.
    const geometry = await session.send('Runtime.evaluate', {
      expression:'({width:innerWidth,height:innerHeight,dpr:devicePixelRatio,scrollX,scrollY})', returnByValue:true,
    });
    if (geometry.exceptionDetails || !geometry.result.value) throw Error('Viewport geometry unavailable');
    const viewport = geometry.result.value;
    const values = {};
    for (const [key,nodeId] of Object.entries(nodes)) {
      const [html,box] = await Promise.all([
        session.send('DOM.getOuterHTML',{nodeId}), session.send('DOM.getBoxModel',{nodeId}),
      ]);
      const q=box.model.border, xs=[q[0],q[2],q[4],q[6]], ys=[q[1],q[3],q[5],q[7]];
      values[key]={value:leadingNumber(html.outerHTML), bounds:[Math.min(...xs),Math.min(...ys),Math.max(...xs)-Math.min(...xs),Math.max(...ys)-Math.min(...ys)],raw_html:html.outerHTML};
    }
    const after=Date.now();
    return {request_ms:before,response_ms:after,time_ms:(before+after)/2,viewport,values,
      geometry_supported:viewport.scrollX===0&&viewport.scrollY===0,
      measurement:'CDP DOM text and boxes; numeric parsing in evaluator process; sequential, not atomic'};
  };
}
