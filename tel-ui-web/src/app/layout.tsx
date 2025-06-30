import type { Metadata } from "next";
import "./globals.css";

export const metadata: Metadata = {
  title: "tel-on-chain - DeFi Liquidity Visualizer",
  description: "Visualize on-chain buy/sell walls and liquidity distribution across DEXes. Identify key support and resistance levels in DeFi markets.",
  keywords: ["DeFi", "liquidity", "DEX", "on-chain", "trading", "support", "resistance"],
  authors: [{ name: "tel-on-chain team" }],
  openGraph: {
    title: "tel-on-chain - DeFi Liquidity Visualizer",
    description: "Visualize on-chain buy/sell walls and liquidity distribution across DEXes",
    type: "website",
  },
};

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <html lang="en">
      <head>
        <link rel="icon" href="/favicon.ico" />
      </head>
      <body className="antialiased bg-gray-50">
        {children}
      </body>
    </html>
  );
}
