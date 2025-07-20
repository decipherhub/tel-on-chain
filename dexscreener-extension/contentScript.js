(() => {
  const TEL_API_BASE = "http://localhost:8081"; // TODO: Configure according to deployment

  // Map chain slug used by Dexscreener to numeric chain IDs used by Tel API
  const slugToChainId = {
    ethereum: 1,
    arbitrum: 42161,
    polygon: 137,
    bsc: 56,
    optimism: 10,
    base: 8453,
    avalanche: 43114,
  };

  // Extract the chain slug and pair address from the current URL
  function getPoolFromUrl(url) {
    try {
      const u = new URL(url);
      const parts = u.pathname.split("/").filter(Boolean);
      if (parts.length >= 2) {
        return {
          chain: parts[0],
          address: parts[1].toLowerCase(),
        };
      }
    } catch (e) {
      // Silent fail – unsupported URL structure
    }
    return null;
  }

  // Query Dexscreener public API to resolve token addresses for a pair
  async function getPairInfo(chainSlug, pairAddress) {
    const apiUrl = `${TEL_API_BASE}/v1/dex/pairs/${chainSlug}/${pairAddress}`;
    try {
      const res = await fetch(apiUrl);
      if (!res.ok) return null;
      const data = await res.json();
      // API returns top-level "pair" object
      return data?.pair ?? null;
    } catch (_) {
      return null;
    }
  }

  // Fetch liquidity wall data from Tel On-Chain API
  async function getLiquidityWalls(token0, token1, chainId) {
    const apiUrl = `${TEL_API_BASE}/v1/liquidity/walls/${token0}/${token1}?chain_id=${chainId}`;
    try {
      const res = await fetch(apiUrl);
      if (!res.ok) return null;
      return await res.json();
    } catch (_) {
      return null;
    }
  }

  // Create or retrieve the overlay container element
  function createOverlay() {
    let overlay = document.getElementById("tel-liquidity-overlay");
    if (overlay) return overlay;

    overlay = document.createElement("div");
    overlay.id = "tel-liquidity-overlay";
    overlay.style.position = "absolute";
    overlay.style.top = "0";
    overlay.style.left = "0";
    overlay.style.pointerEvents = "none";
    overlay.style.width = "100%";
    overlay.style.height = "100%";
    overlay.style.zIndex = "9999";

    document.body.appendChild(overlay);
    return overlay;
  }

  // Render liquidity walls data as a simple floating panel for now
  function renderLiquidityWalls(walls) {
    const overlay = createOverlay();

    // Clear previous content
    overlay.innerHTML = "";

    const panel = document.createElement("div");
    panel.style.background = "rgba(0, 0, 0, 0.8)";
    panel.style.color = "#fff";
    panel.style.padding = "8px";
    panel.style.borderRadius = "4px";
    panel.style.maxHeight = "300px";
    panel.style.overflowY = "auto";
    panel.style.fontSize = "12px";
    panel.style.pointerEvents = "auto";
    panel.style.margin = "8px";
    panel.style.alignSelf = "flex-start";
    panel.style.justifySelf = "flex-end";

    const title = document.createElement("div");
    title.style.fontWeight = "bold";
    title.style.marginBottom = "4px";
    title.textContent = "Tel Liquidity Walls";
    panel.appendChild(title);

    const priceLine = document.createElement("div");
    priceLine.textContent = `Current Price: ${walls.price}`;
    panel.appendChild(priceLine);

    const sellHeader = document.createElement("div");
    sellHeader.style.marginTop = "8px";
    sellHeader.style.fontWeight = "bold";
    sellHeader.textContent = "Sell Walls";
    panel.appendChild(sellHeader);

    walls.sell_walls.forEach((wall) => {
      const div = document.createElement("div");
      div.textContent = `${wall.price_lower.toFixed(
        4
      )} – ${wall.price_upper.toFixed(4)} : ${wall.liquidity_value.toFixed(2)}`;
      panel.appendChild(div);
    });

    const buyHeader = document.createElement("div");
    buyHeader.style.marginTop = "8px";
    buyHeader.style.fontWeight = "bold";
    buyHeader.textContent = "Buy Walls";
    panel.appendChild(buyHeader);

    walls.buy_walls.forEach((wall) => {
      const div = document.createElement("div");
      div.textContent = `${wall.price_lower.toFixed(
        4
      )} – ${wall.price_upper.toFixed(4)} : ${wall.liquidity_value.toFixed(2)}`;
      panel.appendChild(div);
    });

    overlay.appendChild(panel);
  }

  // Main routine: resolve pool → tokens → liquidity walls → render
  async function processPage() {
    const pool = getPoolFromUrl(location.href);
    if (!pool) return;

    const chainId = slugToChainId[pool.chain];
    if (!chainId) return;

    const pairInfo = await getPairInfo(pool.chain, pool.address);
    if (!pairInfo) return;

    const token0 = pairInfo.baseToken.address.toLowerCase();
    const token1 = pairInfo.quoteToken.address.toLowerCase();

    const walls = await getLiquidityWalls(token0, token1, chainId);
    if (!walls) return;

    renderLiquidityWalls(walls);
  }

  // Initial run
  processPage();

  // Observe SPA-navigation (Dexscreener uses pushState)
  let lastHref = location.href;
  setInterval(() => {
    if (location.href !== lastHref) {
      lastHref = location.href;
      processPage();
    }
  }, 2000);
})();
