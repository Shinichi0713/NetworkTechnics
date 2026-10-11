---
title: "独立同分布の仮定のもと、不偏推定量を構築するU-Statics"
emoji: "🙏"
type: "tech" # tech: 技術記事 / idea: アイデア記事
topics: ["statics", "mathematics", "RLVR", "推定理論"]
published: true
---
[LLMの強化学習法であるRLVR](https://zenn.dev/yoshinn/articles/reinforcement_learning_with_vr)の一種である[GRPO](https://yoshishinnze.hatenablog.com/entry/2026/10/12/043000)の性能の数学的な根拠としてU-Staticsが挙げられています。

統計学であり、推定理論の中で重要な概念となっている U-Statics について説明していきます。

## 概要

「uStatics」は統計学における **U-statistics（U統計量）** のことです。これは1948年に Wassily Hoeffding によって導入された、推定理論における統計量の重要なクラスです。[U-statistic](https://en.wikipedia.org/wiki/U_statistic) という名前の「U」は「unbiased（不偏）」に由来すると広く言われています。

### 1. 定義

独立同分布な確率変数 $X_1, X_2, \ldots, X_n \sim F$ に対し、$m$ 変数の対称関数（**カーネル**と呼ばれます）$h(x_1, \ldots, x_m)$ を考えます。ここで $m$ を **次数（degree）** と呼びます。

このとき、パラメータ

$$
\theta(F) = \mathbb{E}[h(X_1, \ldots, X_m)] = \int \cdots \int h(x_1, \ldots, x_m) \, dF(x_1) \cdots dF(x_m)
$$

の不偏推定量として、**U統計量** $U_n$ は次のように定義されます：

$$
U_n = \frac{1}{\binom{n}{m}} \sum_{(i_1, \ldots, i_m) \in \mathcal{C}_{n,m}} h(X_{i_1}, \ldots, X_{i_m})
$$

ここで $\mathcal{C}_{n,m} = \{(i_1, \ldots, i_m) : 1 \leq i_1 < i_2 < \cdots < i_m \leq n\}$ は、$\{1,\ldots,n\}$ から $m$ 個を選ぶすべての組み合わせの集合です。[U Statistics](https://web.stanford.edu/class/stats300b/Slides/16-u-statistics.pdf)

### 2. 具体例

__例1：標本分散__

標本分散 $S_n^2 = \frac{1}{n-1}\sum_{i=1}^n (X_i - \bar{X}_n)^2$ は、実はU統計量として書けます：

$$
S_n^2 = \frac{1}{\binom{n}{2}} \sum_{1 \leq i < j \leq n} \frac{(X_i - X_j)^2}{2}
$$

カーネルは $h(x,y) = \frac{(x-y)^2}{2}$ で、次数 $m=2$ です。[U-statistics](http://publish.illinois.edu/xiaohuichen/files/2018/11/stat05986_Chen_v3_nohighlight.pdf)

__例2：2標本の平均差__

2つの独立な標本 $X_1,\ldots,X_n$ と $Y_1,\ldots,Y_m$ に対し、Mann-WhitneyのU検定に現れる統計量もU統計量です。カーネル $h(x,y) = \mathbf{1}_{\{x \leq y\}}$ を用いて：

$$
U_{n,m} = \frac{1}{nm} \sum_{i=1}^n \sum_{j=1}^m h(X_i, Y_j)
$$

### 3. 不偏性と完備十分性

U統計量の重要な性質は、**不偏性**を保ちながら、分散を最小化することです。

特に、順序統計量が十分統計量であり、かつ分布族が非パラメトリックな場合、U統計量は **最小分散不偏推定量（UMVUE）** となります。[Stat 709: Mathematical Statistics](https://pages.stat.wisc.edu/~shao/stat709/stat709-18.pdf)

### 4. Hoeffding分解

U統計量の漸近理論の核心は **Hoeffding分解** です。$U_n - \theta(F)$ を次数の低いU統計量の和として表現できます：

$$
U_n - \theta(F) = \sum_{c=1}^m \binom{m}{c} U_n^{(c)}
$$

ここで $U_n^{(c)}$ は次数 $c$ のカーネル $h_c$ によるU統計量で、互いに無相関です。[Hoeffding decomposition](https://encyclopediaofmath.org/wiki/Hoeffding_decomposition)

特に、射影（projection）を用いた一次近似：

$$
\hat{U}_n - \theta(F) = \frac{m}{n} \sum_{i=1}^n h_1(X_i)
$$

が重要です。ここで

$$
h_1(x) = \mathbb{E}[h(x, X_2, \ldots, X_m)] - \theta(F)
$$

です。もし $h_1 \equiv 0$ ならばU統計量は **縮退（degenerate）** と呼ばれ、漸近分布が正規分布にならなくなります。[U-statistics](http://publish.illinois.edu/xiaohuichen/files/2018/11/stat05986_Chen_v3_nohighlight.pdf)

### 5. 漸近正規性

非縮退の場合（$\zeta_1 = \mathrm{Var}(h_1(X_1)) > 0$）、Hoeffdingの定理により：

$$
\sqrt{n}(U_n - \theta(F)) \xrightarrow{d} \mathcal{N}\left(0, m^2 \zeta_1\right)
$$

すなわち、適切に標準化したU統計量は正規分布に収束します。これは大数の強法則と中心極限定理をU統計量に拡張したものです。[A Class of Statistics with Asymptotically Normal Distribution](https://exa.ai/library/publication/rgbm97hdljm)

## 研究背景

U-statisticsは、**非パラメトリック統計学（nonparametric statistics）** の中核的な課題を解決するために研究されました。具体的には、以下の3つの大きな問題を扱いました。

### 1. 分布の形を仮定しない状況での「推定可能なパラメータ」の不偏推定

古典的な統計学では、データが正規分布に従うなどの強い仮定を置いてパラメータ推定を行います。しかし、Hoeffdingを含む研究者たちは、**「データの出処について不必要に制限的な仮定を置かずに分析する」** ことを目指しました。[Wassily Hoeffding, June 12, 1914–February 28, 1991](https://nap.nationalacademies.org/resource/biomems/whoeffding.html)

この文脈で生まれたのが **「推定可能なパラメータ（estimable parameter）」** の概念です。パラメータ $\theta(P)$ が推定可能であるとは、ある整数 $m$ と関数 $h$ が存在して

$$
\mathbb{E}_P[h(X_1, \ldots, X_m)] = \theta(P) \quad (\forall P \in \mathcal{P})
$$

が成り立つことを意味します。[U-STATISTICS](https://www.math.ucla.edu/~tom/Stat200C/Ustat.pdf)

例えば、分散 $\sigma^2$ は

$$
\sigma^2 = \mathbb{E}\left[\frac{(X_1 - X_2)^2}{2}\right]
$$

と書けるので、$h(x_1, x_2) = \frac{(x_1 - x_2)^2}{2}$ というカーネルで推定可能です。

### 2. 最小分散不偏推定量（UMVUE）の系統的構成

推定可能なパラメータに対して、**どの不偏推定量よりも分散が小さい（または等しい）推定量を構成する**という課題がありました。

Hoeffdingの理論により、U統計量は次の最適性を持つことが示されました：

> 順序統計量が完備十分統計量となるような非パラメトリックな分布族において、対称な不偏推定量（すなわちU統計量）は **最小分散不偏推定量（UMVUE）** となる。[Stat 709: Mathematical Statistics](https://pages.stat.wisc.edu/~shao/stat709/stat709-18.pdf)

つまり、任意の不偏推定量 $\hat{\theta}$ があれば、それを対称化してU統計量に変換することで、分散を減少（または不変）させつつ不偏性を保つことができるのです。

### 3. 大標本における漸近分布の確立

不偏性だけでは不十分で、**「標本サイズ $n \to \infty$ のとき、推定量はどの分布に近づくか」** という漸近理論が必要でした。

Hoeffdingの1948年の論文 [A Class of Statistics with Asymptotically Normal Distribution](https://exa.ai/library/publication/rgbm97hdljm) は、まさにこの問題に答えました：

$$
\sqrt{n}(U_n - \theta) \xrightarrow{d} \mathcal{N}(0, m^2 \zeta_1)
$$

この結果は、標本平均の中心極限定理をはるかに一般化したものであり、**標本分散、Kendallの順位相関係数 $\tau$、Mann-WhitneyのU統計量、Cramér-von Mises統計量**など、多様な統計量の漸近正規性を統一的に証明できる枠組みを提供しました。

## 基本原理

U-statisticsの基本原理は、**「複数の観測値を組み合わせて平均し、対称性と独立性を最大限に活用して良い推定量を作る」** という考え方です。以下、直感的に段階を追って説明します。

### 1. 問題の出発点：「2つ以上のデータが必要なパラメータ」

まず、推定したいパラメータ $\theta$ が、1つの観測値 $X_i$ だけでは定義できない場合を考えます。

たとえば、**分散** $\sigma^2$ は「データのばらつき」を表す量なので、1つの値 $X_1$ だけでは計算できません。2つの独立なデータ $X_1, X_2$ があれば、

$$
\mathbb{E}\left[\frac{(X_1 - X_2)^2}{2}\right] = \sigma^2
$$

が成り立ちます。つまり、**カーネル**と呼ばれる関数 $h(x_1, x_2) = \frac{(x_1 - x_2)^2}{2}$ を使うと、2つのデータの組み合わせから分散を「作り出せる」のです。

一般に、推定したいパラメータが

$$
\theta = \mathbb{E}[h(X_1, X_2, \ldots, X_m)]
$$

と書けるとき、$h$ を **カーネル（kernel）**、$m$ を **次数（degree）** と呼びます。

### 2. ナイーブな推定量の問題：重複を含むと偏る

さて、$n$ 個のデータ $X_1, \ldots, X_n$ があるとき、$\theta$ を推定する最も素朴な方法は、**すべての $m$ 個の組み合わせ（順列）** で $h$ を計算して平均することです：

$$
V_n = \frac{1}{n^m} \sum_{i_1=1}^n \cdots \sum_{i_m=1}^n h(X_{i_1}, \ldots, X_{i_m})
$$

これを **V-statistic（von Mises統計量）** と呼びます。しかし、これには大きな問題があります。

__問題：同じデータを繰り返し使うと、期待値が $\theta$ にならない__

例えば $m=2$ のとき、$i_1 = i_2$ となる項（つまり $h(X_1, X_1)$ のような項）が含まれます。これらの項は独立なデータの組み合わせではないため、期待値が $\theta$ からずれてしまい、**推定量に偏り（bias）** が生じます。

### 3. U-statisticsの解決策：「重複なしの組み合わせ」で平均する

Hoeffdingの発想はシンプルです：

> **同じデータを2回以上使わないように、異なる添字の組み合わせだけを平均すればよい。**

すなわち、$1 \leq i_1 < i_2 < \cdots < i_m \leq n$ となるすべての組み合わせについて平均を取ります：

$$
U_n = \frac{1}{\binom{n}{m}} \sum_{1 \leq i_1 < \cdots < i_m \leq n} h(X_{i_1}, \ldots, X_{i_m})
$$

これが **U-statistics** です。「U」は **Unbiased（不偏）** の頭文字です。

__なぜこれで不偏になるのか？__

各項 $h(X_{i_1}, \ldots, X_{i_m})$ は、添字がすべて異なるので、独立同分布な $m$ 個の標本に対するカーネルの値です。したがって

$$
\mathbb{E}[U_n] = \frac{1}{\binom{n}{m}} \sum_{1 \leq i_1 < \cdots < i_m \leq n} \mathbb{E}[h(X_{i_1}, \ldots, X_{i_m})] = \frac{1}{\binom{n}{m}} \cdot \binom{n}{m} \cdot \theta = \theta
$$

となり、**厳密に不偏** になります。

### 4. 対称化の原理：順番は関係ない

カーネル $h$ は、もともと対称でなくても構いません。例えば $f(x_1, x_2) = x_1^2 x_2$ は対称ではありませんが、

$$
h(x_1, x_2) = \frac{f(x_1, x_2) + f(x_2, x_1)}{2} = \frac{x_1^2 x_2 + x_2^2 x_1}{2}
$$

と対称化すれば、期待値は変わらず、組み合わせ $\{i,j\}$ に対して一意な値を与えます。

一般に、$m!$ 個の順列すべてで平均すれば、対称カーネルが得られます：

$$
h(x_1, \ldots, x_m) = \frac{1}{m!} \sum_{\pi \in \Pi_m} f(x_{\pi_1}, \ldots, x_{\pi_m})
$$

これにより、**どの順番でデータを選んでも同じ値が出る**ようになり、統計量が標本の順序に依存しないという自然な性質が保証されます。

### 5. なぜ分散が小さいのか？：重複の排除と「相関の制御」

U-statisticsが単なる不偏推定量ではなく、**最小分散不偏推定量（UMVUE）** になりうる理由は、データの重複を排除することで、項同士の相関を最小限に抑えていることにあります。

$U_n$ の分散を考えます。2つの項 $h(X_{i_1}, \ldots, X_{i_m})$ と $h(X_{j_1}, \ldots, X_{j_m})$ が共通する添字を $c$ 個持つとき、その共分散は $\zeta_c$ と書かれます。

$$
\mathrm{Var}(U_n) = \sum_{c=1}^m \frac{\binom{m}{c}\binom{n-m}{m-c}}{\binom{n}{m}} \zeta_c
$$

ここで重要なのは、$c$（共通添字数）が大きいほど共分散 $\zeta_c$ の寄与が大きくなる一方、**$n$ が大きいときは $c=1$ の項が支配的** となることです。つまり、

$$
\mathrm{Var}(U_n) \approx \frac{m^2}{n} \zeta_1 \quad (n \to \infty)
$$

となり、$n$ に反比例して分散が減少します。これは「うまく設計された平均」が持つ性質であり、**すべてのデータを公平に使いつつ、過剰な相関を避けている**からです。

### 6. Hoeffding分解の直感：複雑な統計量を「足し算」に分解する

U-statisticsの最も美しい原理の一つが **Hoeffding分解** です。これは、$U_n - \theta$ を「複雑さの違う」項の和に分解するものです：

$$
U_n - \theta = \sum_{c=1}^m \binom{m}{c} U_n^{(c)}
$$

ここで $U_n^{(c)}$ は「$c$ 個のデータの相互作用」だけを捉えるU統計量です。

__直感的な意味__

- $c=1$ の項：各 $X_i$ 単体の効果（線形部分）
- $c=2$ の項：ペア $(X_i, X_j)$ の相互作用（二次部分）
- $c=3$ の項：トリプレットの相互作用（三次部分）
- $\vdots$

この分解により、**U-statisticsの漸近分布は $c=1$ の項（射影）だけで決まる**ことがわかります。これが漸近正規性の証明の核心です：

$$
\sqrt{n}(U_n - \theta) \approx \frac{m}{\sqrt{n}} \sum_{i=1}^n h_1(X_i) \xrightarrow{d} \mathcal{N}(0, m^2 \zeta_1)
$$

ここで $h_1(x) = \mathbb{E}[h(x, X_2, \ldots, X_m)] - \theta$ は、1つのデータ $x$ が $\theta$ への寄与を表す関数です。

### 7. 具体例で確認：標本分散

基本原理を標本分散で確認します。

__カーネルの導出__

分散は $\sigma^2 = \mathbb{E}[X^2] - (\mathbb{E}[X])^2$ と書けます。2つの独立なデータ $X_1, X_2$ を使って：

$$
\mathbb{E}\left[\frac{(X_1 - X_2)^2}{2}\right] = \frac{1}{2}\mathbb{E}[X_1^2 - 2X_1X_2 + X_2^2] = \frac{1}{2}(2\mathbb{E}[X^2] - 2(\mathbb{E}X)^2) = \sigma^2
$$

したがってカーネルは $h(x_1, x_2) = \frac{(x_1 - x_2)^2}{2}$ です。

__U-statisticsとしての標本分散__

$$
U_n = \frac{1}{\binom{n}{2}} \sum_{1 \leq i < j \leq n} \frac{(X_i - X_j)^2}{2} = \frac{1}{n(n-1)} \sum_{i<j} (X_i - X_j)^2
$$

これを展開すると：

$$
\sum_{i<j} (X_i - X_j)^2 = \sum_{i<j} \left[(X_i - \bar{X}) - (X_j - \bar{X})\right]^2 = n \sum_{i=1}^n (X_i - \bar{X})^2
$$

よって、

$$
U_n = \frac{1}{n(n-1)} \cdot n \sum_{i=1}^n (X_i - \bar{X})^2 = \frac{1}{n-1} \sum_{i=1}^n (X_i - \bar{X})^2 = S_n^2
$$

つまり、**標本分散 $S_n^2$ はまさにU-statisticsの形をしている**のです。なぜ分母が $n-1$ になるのかという疑問も、U-statisticsの枠組みでは「異なる添字の組み合わせ数 $\binom{n}{2}$ で正規化するから」という一貫した理由で説明できます。

## 出来ること/出来ないこと

U-statisticsのできることとできないことを、理論的性質と実用上の制約の両面からまとめます。

### U-statistics でできること

__1. 推定可能なパラメータの不偏推定__

パラメータ $\theta$ が **推定可能（estimable）** であるとは、あるカーネル $h$ と次数 $m$ が存在して

$$
\theta = \mathbb{E}[h(X_1, \ldots, X_m)]
$$

と書けることを意味します。このような $\theta$ に対し、U-statistics

$$
U_n = \frac{1}{\binom{n}{m}} \sum_{1 \leq i_1 < \cdots < i_m \leq n} h(X_{i_1}, \ldots, X_{i_m})
$$

は常に $\theta$ の **不偏推定量** となります。[U-STATISTICS](https://www.math.ucla.edu/~tom/Stat200C/Ustat.pdf)

さらに、順序統計量が完備十分であるような非パラメトリック分布族では、U-statistics は **最小分散不偏推定量（UMVUE）** となります。つまり、どの不偏推定量よりも分散が小さい（または等しい）最良の推定量です。[Stat 709: Mathematical Statistics](https://pages.stat.wisc.edu/~shao/stat709/stat709-18.pdf)

__2. 多様な統計量を統一的な枠組みで扱う__

U-statistics の枠組みは、一見異なる統計量を同じ形式で理解できる統一理論を提供します。

| 統計量 | カーネル $h$ | 次数 $m$ | 推定するパラメータ |
|--------|-------------|---------|------------------|
| 標本平均 | $h(x) = x$ | 1 | $\mu = \mathbb{E}[X]$ |
| 標本分散 | $h(x,y) = \frac{(x-y)^2}{2}$ | 2 | $\sigma^2 = \mathrm{Var}(X)$ |
| Gini平均差 | $h(x,y) = \|x - y\|$ | 2 | $\mathbb{E}\|X - X'\|$ |
| Kendall's $\tau$ | $h = 2 \cdot \mathbf{1}_{x_1<x_2, y_1<y_2} - 1$ | 2 | 順位相関 |
| Mann-Whitney | $h(x,y) = \mathbf{1}_{x \leq y}$ | (1,1) | $P(X \leq Y)$ |
| Wilcoxon符号順位 | $h(z_1, z_2) = \mathbf{1}_{z_1+z_2>0}$ | 2 | $P(X_1+X_2>0)$ |

このように、**検定統計量・相関係数・分散推定**などがすべて同じ「U-statistics」という共通言語で記述できます。[U Statistics](https://people.eecs.berkeley.edu/~jordan/courses/210B-spring08/lectures/stat210b_lecture_3.pdf)

__3. 漸近正規性の一般定理__

非縮退（nondegenerate）の条件下で、U-statistics は正規分布に収束します：

$$
\sqrt{n}(U_n - \theta) \xrightarrow{d} \mathcal{N}(0, m^2 \zeta_1)
$$

ここで $\zeta_1 = \mathrm{Var}(h_1(X_1))$ であり、$h_1(x) = \mathbb{E}[h(x, X_2, \ldots, X_m)] - \theta$ です。

これは標本平均の中心極限定理を **次数 $m$ のカーネルまで一般化** したものであり、個別の統計量ごとに漸近分布を証明する必要がなくなります。[A Class of Statistics with Asymptotically Normal Distribution](https://exa.ai/library/publication/rgbm97hdljm)

__4. Hoeffding分解による構造分析__

U-statistics は次のように分解できます：

$$
U_n - \theta = \sum_{c=1}^m \binom{m}{c} U_n^{(c)}
$$

各 $U_n^{(c)}$ は「$c$ 個のデータの相互作用」を表し、互いに無相関です。これにより：

- どの次数の相互作用が支配的かを特定できる
- 漸近分布が $c=1$ の項（線形射影）で決まることを証明できる
- **縮退（degenerate）の度合い**を形式的に定義できる

__5. 2標本・多標本問題への拡張__

独立な標本 $\{X_1,\ldots,X_{n_1}\}$ と $\{Y_1,\ldots,Y_{n_2}\}$ に対しても、

$$
U_{n_1,n_2} = \frac{1}{\binom{n_1}{r}\binom{n_2}{s}} \sum h(X_{i_1},\ldots,X_{i_r}, Y_{j_1},\ldots,Y_{j_s})
$$

という形で拡張でき、Mann-Whitney検定などの2標本問題を統一的に扱えます。

### U-statistics でできないこと・限界

__1. 計算量の爆発（組み合わせ爆発）__

U-statistics の最大の実用上の弱点は、**すべての $m$ 個の組み合わせを平均する**必要があることです。項の数は $\binom{n}{m}$ であり、例えば $m=3$, $n=10000$ なら

$$
\binom{10000}{3} \approx 1.67 \times 10^{11}
$$

となり、現実的な計算時間では処理できません。[DESIGN BASED INCOMPLETE U-STATISTICS](https://www3.stat.sinica.edu.tw/statistica/oldpdf/A31n322.pdf)

特に **高次（high-order）U-statistics**（$m \geq 3$）は、統計的には有用でも計算上は困難で、これを「統計的・計算的トレードオフ（statistical-computational trade-off）」と呼びます。[U-Statistic Reduction](https://arxiv.org/pdf/2306.03793)

__2. 独立同分布（i.i.d.）仮定への強い依存__

U-statistics の理論の大部分は **$X_1,\ldots,X_n$ が i.i.d.** であることを前提としています。データに依存構造がある場合：

- 時系列データ（自己相関がある）
- 空間データ（近傍同士が相関している）
- ネットワークデータ（ノード間に依存関係がある）

には、漸近正規性や不偏性が一般に成立しません。依存データに対する拡張は **mixing条件** や **exchangeability** などの追加仮定が必要で、理論はかなり複雑になります。[Deriving the asymptotic distribution of U and V-statistics of dependent data](https://projecteuclid.org/journalArticle/Download?urlId=10.3150%2F11-BEJ358)

__3. 縮退（Degenerate）した場合の非正規漸近分布__

$h_1(x) \equiv 0$、つまり

$$
\mathbb{E}[h(x, X_2, \ldots, X_m)] = \theta \quad (\forall x)
$$

となる場合、U-statistics は **1次縮退（degenerate of order 1）** と呼ばれます。このとき、線形射影項が消えてしまい、漸近分布は **正規分布ではなく、Wiener chaos（または多重積分の二次形式）** に収束します。

例えば、分散が0に近い状況や、帰無仮説が厳密に成立する場合にこの現象が起こります。これは推定・検定の設計を難しくする要因です。[Degenerate U- and V-statistics under weak dependence](https://projecteuclid.org/journals/bernoulli/volume-18/issue-2/Degenerate-U--and-V-statistics-under-weak-dependence/10.3150/11-BEJ354.pdf)

__4. すべてのパラメータが推定可能とは限らない__

U-statistics は **推定可能なパラメータ** $\theta(P) = \mathbb{E}_P[h(X_1,\ldots,X_m)]$ にしか適用できません。例えば：

- 密度関数 $f(x)$ そのものの点推定
- 分布の支持集合（support）の境界
- 複雑な汎関数 $\theta(P)$ で、どのような $h$ でも $\mathbb{E}[h] = \theta$ とならないもの

はU-statisticsの枠組みに入りません。[U-STATISTICS](https://www.math.ucla.edu/~tom/Stat200C/Ustat.pdf)

__5. U-process の最小化は難しい__

U-statistics をパラメータ $\alpha$ でインデックスされた族 $\{U_n(\alpha)\}_{\alpha \in \mathcal{A}}$ とみなしたものを **U-process** と呼びます。これを最小化する問題：

$$
\hat{\alpha} = \arg\min_{\alpha} U_n(\alpha)
$$

は、経験リスク最小化（ERM）の一般化として現れますが、**U-process は i.i.d. 和ではない**ため、通常の統計的学習理論の道具（VC次元、Rademacher複雑度など）をそのまま適用できません。一様大数法・一様中心極限定理の拡張が必要で、理論的・計算的に困難です。[Lecture 4: U-Statistics & U-Process Minimizers](https://eml.berkeley.edu/~bgraham/Teaching/CEMFI_August2015/Lecture_4_U_Statistics.pdf)

__6. 大規模データ・分散環境での困難さ__

現代の機械学習の文脈では、データが分散して保存されている場合や、ストリーミングデータの場合があります。U-statistics は **すべての $m$ 個の組み合わせを同時に扱う必要がある**ため、分散計算やオンライン計算に不向きです。

この問題に対処するために **incomplete U-statistics**（ランダムサンプリングで一部の組み合わせだけ計算）や **distributed U-statistics** が研究されていますが、これらは近似であり、元のU-statisticsの厳密な性質をすべて保持するわけではありません。[Scaling-up Empirical Risk Minimization: Optimization of Incomplete U-statistics](https://researchers.lille.inria.fr/abellet/papers/jmlr16.pdf)

## 総括
U-statisticsの本質は、**「独立同分布という最も素朴な構造の上に、組み合わせの対称性を通じて普遍に成立する推定量を構築し、それをHoeffding分解によって還元的に理解する」** という統計思想にあります。以下、記事の内容を貫く4つの本質的洞察として総括します。

### 1. 本質は「重複排除」という設計思想にある

U-statisticsの最も核心的な発想は、単なる「平均」を超えた **「独立な情報だけを公平に集約する」** という設計にあります。

ナイーブなV-statistic（von Mises統計量）はすべての $m$ 個の順列を平均しますが、同じ添字が重複する項（例：$h(X_1, X_1)$）を含むため、期待値がずれます。Hoeffdingは、**異なる添字の組み合わせだけ**を選んで平均することで、この偏りを根本的に排除しました：

$$
U_n = \frac{1}{\binom{n}{m}} \sum_{1 \leq i_1 < \cdots < i_m \leq n} h(X_{i_1}, \ldots, X_{i_m})
$$

この「重複排除」は単なる計算上の工夫ではなく、**統計量の情報構造に関する哲学的選択**です。同じ観測値を2回使うことは、統計的には「偽の独立情報」を作り出す行為であり、U-statisticsはそれを潔く拒否します。結果として、

$$
\mathbb{E}[U_n] = \theta
$$

が厳密に成立し、かつ項間の相関が最小限に抑えられるため、分散も自然に小さくなります。

### 2. 本質は「対称性による普遍化」にある

U-statisticsの美しさは、**個別の統計量の特殊性を対称性の普遍性に還元する**点にあります。

標本分散、Kendallの$\tau$、Mann-Whitney統計量、Wilcoxon符号順位検定など、一見無関係に見える統計量が、すべて

$$
U_n = \text{（対称カーネル } h \text{ の全組み合わせ上の平均）}
$$

という同一の形に収斂します。これは、データの順序に依存しない「本質的な量」だけを抽出する操作です。カーネルの対称化：

$$
h(x_1, \ldots, x_m) = \frac{1}{m!} \sum_{\pi \in \Pi_m} f(x_{\pi_1}, \ldots, x_{\pi_m})
$$

は、「順番に依存しない真理」だけを残し、観測の偶然性（どの順番でデータが来たか）を統計量から排除します。

この普遍化の力は、個別の定理を一つずつ証明する必要をなくし、**「すべてのU-statisticsは同じ漸近正規性を持つ」**という統一的な真理を与えます：

$$
\sqrt{n}(U_n - \theta) \xrightarrow{d} \mathcal{N}(0, m^2 \zeta_1)
$$

個別の統計量の漸近分布を個別に導くのではなく、**「対称カーネルの平均」という抽象度の高いクラス全体に一括で適用できる定理**を持つことが、U-statistics理論の最大の価値です。

### 3. 本質はHoeffding分解による「還元主義」にある

U-statisticsの理論的深みは、**複雑なものを単純な要素の重ね合わせとして理解する** Hoeffding分解に凝縮されています。

$$
U_n - \theta = \sum_{c=1}^m \binom{m}{c} U_n^{(c)}
$$

この分解の本質的な意味は、**「$m$ 個のデータの相互作用は、$1$ 個の効果、$2$ 個の効果、……、$m$ 個の効果の直和に分解できる」**ということです。そして各 $U_n^{(c)}$ は互いに無相関です。

大標本の極限では、$c=1$ の項（線形射影）だけが支配的になります：

$$
\sqrt{n}(U_n - \theta) \approx \frac{m}{\sqrt{n}} \sum_{i=1}^n h_1(X_i)
$$

つまり、**「複雑な $m$ 次の統計量は、漸近的には単純な標本平均の定数倍に還元される」**のです。これは統計学における還元主義の極致であり、「複雑な構造を持つ推定量でも、十分大きな標本では正規分布という最も単純な形に帰着する」という深い真理を示しています。

同時に、この分解は「縮退（degenerate）」という現象も明らかにします。$h_1 \equiv 0$ のとき、$c=1$ の項が消えてしまい、漸近分布は正規ではなくなります。これは **「一次の効果がないとき、高次の相互作用が支配する」** という構造的転換を示唆しており、U-statisticsの理論が単なる「正規近似の道具」ではなく、**データの相互作用の階層構造を解明する理論**であることを物語ります。

### 4. 本質的限界は「i.i.d.の理想と計算の現実の間の緊張」にある

U-statisticsの本質を語る上で、**「できること」と「できないこと」の境界**こそが理論の輪郭を定めています。

__i.i.d.という理想__

U-statisticsのすべての美しい性質（不偏性、UMVUE、漸近正規性、Hoeffding分解）は、**$X_1, \ldots, X_n$ が独立同分布である**という仮定の上に成り立っています。これは「すべての観測値が等価で、どの組み合わせも同じ構造を持つ」という統計学的理想の表れです。しかし現実のデータは時系列相関やネットワーク依存を持ち、この理想は必ずしも成立しません。

__計算の現実__

一方で、組み合わせ数 $\binom{n}{m}$ の爆発は、**「理論的に美しい推定量が、現実には計算不可能である」** という残酷な事実を突きつけます。$m=3$, $n=10000$ で約 $10^{11}$ 項という計算量は、統計的合理性と計算可能性の間の根本的なトレードオフを示しています。

この緊張こそが、現代のU-statistics研究（incomplete U-statistics、分散アルゴリズム、テンソル計算による高速化）を駆動する原動力です。[On computing and the complexity of computing higher-order U-statistics, exactly](https://arxiv.org/pdf/2508.12627)

### 結論：U-statisticsとは何か

U-statisticsは、**「独立同分布という最小限の仮定の上に、対称性と重複排除という設計原理で最良の不偏推定量を構築し、Hoeffding分解によってその漸近構造を還元的に理解する統計学の基盤理論」** です。

その本質は数式に凝縮されています：

$$
\boxed{U_n = \frac{1}{\binom{n}{m}} \sum_{1 \leq i_1 < \cdots < i_m \leq n} h(X_{i_1}, \ldots, X_{i_m}) \approx \theta + \frac{m}{n} \sum_{i=1}^n h_1(X_i) \quad (n \to \infty)}
$$

左辺は「すべての組み合わせの対称な平均」という複雑な統計量であり、右辺は「単純な標本平均の定数倍」という単純な形です。**「複雑な対称性の中に単純な構造が潜んでいる」**ことを示すこの等式こそが、U-statisticsの本質です。
