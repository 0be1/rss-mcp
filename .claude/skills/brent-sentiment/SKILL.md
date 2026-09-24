---
name: brent-sentiment
description: Note l'impact probable de chaque article d'un flux RSS sur le cours du Brent (score de -1 baisse à +1 hausse) et présente le résultat dans un tableau avec la date de publication ISO 8601. Utilise cette skill dès que l'utilisateur veut savoir si l'actualité pousse le pétrole, le brut ou le Brent (ou le WTI) à la hausse ou à la baisse, demande un sentiment de marché, un score haussier/baissier ou une revue de presse pétrole à partir d'un flux RSS, même s'il ne mentionne pas explicitement « Brent » ou « score ». Also triggers on English requests like "oil news sentiment", "is this bullish or bearish for crude".
argument-hint: "[nom du flux, défaut : oil price]"
allowed-tools: mcp__rss__list_feeds, mcp__rss__get_feed
---

# Impact des articles d'un flux RSS sur le Brent

Flux à analyser : `$ARGUMENTS`. S'il est vide, utilise `oil price`.

Le but est de donner à l'utilisateur une lecture rapide du flux d'actualités pétrolières : quels articles comptent pour le prix du Brent, dans quel sens, et quel est le signal global. Ces informations servent à prendre une décision, donc la justification de chaque score compte autant que le score lui-même.

## Étapes

1. Appelle `list_feeds` pour vérifier que le flux existe. La liste des flux vient d'un fichier de configuration qui peut changer. Si le nom demandé est introuvable, montre les flux disponibles et arrête-toi, au lieu de deviner.
2. Appelle `get_feed` sur ce flux **sans `limit`**, pour analyser tous les articles que le flux renvoie. L'utilisateur veut une vue d'ensemble, pas seulement les derniers titres.
3. Attribue à chaque article un score entre -1 et +1, avec une décimale.

## Barème

Demande-toi : *si le marché ne retenait que cet article, le Brent monterait-il ou baisserait-il ?*

- **Vers +1 (hausse)** : offre coupée ou menacée (blocage d'Ormuz, oléoduc à l'arrêt, sanctions, attaques), escalade géopolitique, échec de négociations, stocks en baisse, demande en hausse.
- **Vers -1 (baisse)** : offre rétablie ou en hausse (redémarrage, contournements, hausse de production), désescalade ou accord de paix, recul de la demande (ralentissement économique, électrification des transports).
- **0 (neutre)** : article sans lien direct avec l'offre ou la demande de brut. Par exemple le gaz ou le GNL seul, l'électricité, des fusions-acquisitions ou une technologie sans effet à court terme.

Réserve ±0.7 à ±1 aux articles qui décrivent un choc direct et important sur l'offre ou la demande. Les signaux indirects, hypothétiques ou mixtes restent entre ±0.1 et ±0.4. Un article peut contenir des éléments dans les deux sens : fais le bilan net et mentionne la tension dans la raison.

## Format de sortie

Un tableau Markdown, trié du plus récent au plus ancien :

| # | Date de publication (ISO 8601) | Article | Score Brent | Raison |
|---|---|---|---|---|
| 1 | 2026-09-24T14:15:00+00:00 | [Titre](lien) | **-0.3** | Une phrase courte en français |

- Reprends la date telle quelle depuis le champ `published` : elle est déjà au format ISO 8601. Si elle est absente, écris `—`.
- Laisse le titre dans sa langue d'origine et mets-le en lien vers l'article.
- La raison et le reste de la réponse sont en français.

Après le tableau :

- donne la **moyenne des scores**, suivie d'une phrase qui résume le signal global et les articles qui le portent ;
- signale les contradictions entre articles (par exemple une infrastructure décrite comme à la fois en service et à l'arrêt), car elles changent la lecture du signal ;
- précise que l'analyse repose sur les résumés du flux, souvent tronqués, et non sur les articles complets.
