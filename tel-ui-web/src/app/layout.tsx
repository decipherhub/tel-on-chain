import type { Metadata } from "next";
import "./globals.css";

export const metadata: Metadata = {
  title: "tel-on-chain | DeFi Liquidity Analytics",
  description: "Visualize on-chain buy/sell walls and liquidity distribution across DEXes. Identify key support and resistance levels in DeFi markets.",
  keywords: ["DeFi", "liquidity", "DEX", "on-chain", "trading", "Uniswap", "analytics"],
  authors: [{ name: "tel-on-chain" }],
  openGraph: {
    title: "tel-on-chain | DeFi Liquidity Analytics",
    description: "Real-time liquidity visualization across multiple DEXes",
    type: "website",
  },
};

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <html lang="en" className="dark">
      <head>
        <link rel="icon" href="/favicon.ico" />
      </head>
      <body className="antialiased min-h-screen">
        {children}
      </body>
    </html>
  );
}
