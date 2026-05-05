import os
import logging
import re
from datetime import datetime, timezone

import aiohttp
import discord
import numpy as np
from discord.ext import tasks
from dotenv import load_dotenv

import spam_filter
import docs_index
from llm import GeminiClient, QuotaExceeded

load_dotenv()

logging.basicConfig(level=logging.INFO, format="%(asctime)s [%(levelname)s] %(message)s")
log = logging.getLogger("lean-ctx-bot")

GITHUB_REPO = "yvgude/lean-ctx"
STATS_API = "https://leanctx.com/api/stats"

UPDATE_INTERVAL_MINUTES = 10
KNOWLEDGE_DIR = os.environ.get("KNOWLEDGE_DIR", "knowledge")
MAX_REPLY_LENGTH = 1900

SPAM_WARNING = (
    "⚠️ Your message was detected as spam and has been removed. "
    "Please read our server rules. "
    "If you have questions, reach out to the moderators."
)


def format_number(n: int) -> str:
    if n >= 1_000_000:
        return f"{n / 1_000_000:.1f}M"
    if n >= 10_000:
        return f"{n / 1_000:.1f}k"
    return f"{n:,}".replace(",", "'")


class LeanCtxBot(discord.Client):
    def __init__(self, *, use_privileged_intents: bool = True):
        intents = discord.Intents.default()
        self.spam_protection_enabled = False
        if use_privileged_intents:
            intents.message_content = True
            intents.members = True
            self.spam_protection_enabled = True
        super().__init__(intents=intents)

        self.guild_id = int(os.environ["GUILD_ID"])
        self.ch_stars = int(os.environ["CHANNEL_STARS"])
        self.ch_installs = int(os.environ["CHANNEL_INSTALLS"])
        self.ch_mod_log = int(os.environ.get("CHANNEL_MOD_LOG", "0"))
        self.session: aiohttp.ClientSession | None = None

        self.gemini: GeminiClient | None = None
        self.index: docs_index.DocsIndex | None = None

    async def setup_hook(self):
        self.session = aiohttp.ClientSession()
        self.update_stats.start()
        await self._init_docs_bot()

    async def _init_docs_bot(self):
        """Initialize the RAG docs bot (Gemini + knowledge index)."""
        api_key = os.environ.get("GEMINI_API_KEY")
        if not api_key:
            log.warning("GEMINI_API_KEY not set — docs bot disabled")
            return

        try:
            self.gemini = GeminiClient(api_key)

            cached = docs_index.DocsIndex.load_cached(KNOWLEDGE_DIR)
            if cached and cached.is_ready:
                self.index = cached
                log.info("Docs bot ready (cached index: %d chunks)", len(self.index.chunks))
                return

            chunks = docs_index.load_and_chunk(KNOWLEDGE_DIR)
            if not chunks:
                log.error("No knowledge documents found in %s", KNOWLEDGE_DIR)
                return

            texts = [c.text for c in chunks]
            embeddings = await self.gemini.embed(texts)

            self.index = docs_index.DocsIndex(
                chunks=chunks,
                embeddings=np.array(embeddings, dtype=np.float32),
            )
            self.index.save(KNOWLEDGE_DIR)
            log.info("Docs bot ready (%d chunks indexed)", len(chunks))

        except Exception as e:
            log.error("Failed to initialize docs bot: %s", e)
            self.gemini = None
            self.index = None

    async def on_ready(self):
        log.info("Bot online as %s", self.user)
        if self.spam_protection_enabled:
            log.info("Spam protection ACTIVE (mod-log channel: %s)", self.ch_mod_log or "disabled")
        else:
            log.warning(
                "Spam protection DISABLED — enable MESSAGE CONTENT and SERVER MEMBERS "
                "intents at https://discord.com/developers/applications/"
            )
        if self.index and self.index.is_ready:
            log.info("Docs bot ACTIVE (%d chunks)", len(self.index.chunks))
        else:
            log.warning("Docs bot DISABLED — check GEMINI_API_KEY and knowledge/ directory")

    async def close(self):
        if self.session:
            await self.session.close()
        await super().close()

    # ── Stats ──────────────────────────────────────────────────────

    async def fetch_stats(self) -> tuple[int, int]:
        """Fetch stars and total installs from leanctx.com/api/stats."""
        async with self.session.get(STATS_API) as resp:
            if resp.status != 200:
                log.warning("Stats API %s: %s", resp.status, await resp.text())
                return -1, -1
            data = await resp.json()
            return data.get("stars", -1), data.get("installs", -1)

    async def _rename(self, channel_id: int, name: str):
        guild = self.get_guild(self.guild_id)
        if not guild:
            log.error("Guild %s not found", self.guild_id)
            return
        channel = guild.get_channel(channel_id)
        if not channel:
            log.error("Channel %s not found", channel_id)
            return
        if channel.name == name:
            return
        try:
            await channel.edit(name=name)
            log.info("Updated: %s", name)
        except discord.HTTPException as e:
            log.error("Failed to rename channel %s: %s", channel_id, e)

    @tasks.loop(minutes=UPDATE_INTERVAL_MINUTES)
    async def update_stats(self):
        log.info("Fetching stats...")
        stars, installs = await self.fetch_stats()
        if stars >= 0:
            await self._rename(self.ch_stars, f"⭐ Stars: {format_number(stars)}")
        if installs >= 0:
            await self._rename(self.ch_installs, f"📦 Installs: {format_number(installs)}")
        log.info("Stats update complete.")

    @update_stats.before_loop
    async def before_update(self):
        await self.wait_until_ready()

    # ── Spam Protection ────────────────────────────────────────────

    async def _is_first_message(self, message: discord.Message) -> bool:
        """Check if this is the member's first message in the guild."""
        channel: discord.TextChannel
        for channel in message.guild.text_channels:
            try:
                async for msg in channel.history(limit=5):
                    if msg.author.id == message.author.id and msg.id != message.id:
                        return False
            except discord.Forbidden:
                continue
        return True

    async def _log_spam(self, message: discord.Message, verdict: spam_filter.SpamVerdict):
        """Send a mod-log embed with details about the deleted spam message."""
        if not self.ch_mod_log:
            return
        guild = self.get_guild(self.guild_id)
        if not guild:
            return
        mod_channel = guild.get_channel(self.ch_mod_log)
        if not mod_channel:
            return

        content_preview = message.content[:500]
        if len(message.content) > 500:
            content_preview += "…"

        account_age = (datetime.now(timezone.utc) - message.author.created_at).days

        embed = discord.Embed(
            title="Spam Removed",
            color=discord.Color.red(),
            timestamp=datetime.now(timezone.utc),
        )
        embed.add_field(name="User", value=f"{message.author} ({message.author.id})", inline=True)
        embed.add_field(name="Channel", value=message.channel.mention, inline=True)
        embed.add_field(name="Account Age", value=f"{account_age} days", inline=True)
        embed.add_field(
            name="Score",
            value=f"{verdict.score}/{verdict.threshold}",
            inline=True,
        )
        embed.add_field(name="Reasons", value=verdict.summary, inline=False)
        embed.add_field(name="Message", value=f"```\n{content_preview}\n```", inline=False)

        if message.author.avatar:
            embed.set_thumbnail(url=message.author.avatar.url)

        try:
            await mod_channel.send(embed=embed)
        except discord.HTTPException as e:
            log.error("Failed to send mod log: %s", e)

    # ── Docs Bot (RAG) ─────────────────────────────────────────────

    def _extract_question(self, message: discord.Message) -> str | None:
        """Strip the @mention and extract the actual question."""
        content = message.content
        content = re.sub(r"<@[!&]?\d+>", "", content).strip()
        if len(content) < 3:
            return None
        return content

    async def _handle_docs_question(self, message: discord.Message):
        """Answer a question using the RAG pipeline."""
        question = self._extract_question(message)
        if not question:
            await message.reply(
                "Ask me anything about lean-ctx! For example:\n"
                "`@LeanCTX How do I install lean-ctx?`"
            )
            return

        async with message.channel.typing():
            try:
                query_embedding = await self.gemini.embed_query(question)
                relevant_chunks = self.index.search(query_embedding, top_k=5)

                if not relevant_chunks:
                    await message.reply(
                        "I couldn't find anything about that in the documentation.\n"
                        "Check here: https://leanctx.com/docs/getting-started"
                    )
                    return

                context = [
                    {"text": c.text, "title": c.title, "url": c.url}
                    for c in relevant_chunks
                ]
                answer = await self.gemini.answer(question, context)

                if len(answer) > MAX_REPLY_LENGTH:
                    answer = answer[:MAX_REPLY_LENGTH] + "…"

                await message.reply(answer)

            except QuotaExceeded as e:
                log.warning("Quota exceeded: %s", e)
                await message.reply(
                    "I've reached my daily question limit to keep costs at zero. "
                    "Please try again tomorrow or check the docs: https://leanctx.com/docs/getting-started"
                )

            except Exception as e:
                log.error("Docs bot error: %s", e)
                await message.reply(
                    "Sorry, something went wrong. "
                    "Please try again or check the docs: https://leanctx.com/docs/getting-started"
                )

    # ── Message Router ─────────────────────────────────────────────

    async def on_message(self, message: discord.Message):
        log.info("MSG from %s in %s: %s", message.author, getattr(message.channel, 'name', 'DM'), message.content[:80])
        if message.author.bot:
            return
        if not message.guild:
            return
        if message.guild.id != self.guild_id:
            return

        is_mentioned = self.user and self.user.mentioned_in(message) and not message.mention_everyone

        if not is_mentioned and message.role_mentions and message.guild:
            bot_member = message.guild.get_member(self.user.id)
            if bot_member:
                bot_role_ids = {r.id for r in bot_member.roles}
                is_mentioned = any(r.id in bot_role_ids for r in message.role_mentions)

        if is_mentioned:
            log.info("Bot mentioned by %s in #%s: %s", message.author, message.channel, message.content[:100])
            if self.gemini and self.index and self.index.is_ready:
                log.info("Docs bot handling question...")
                await self._handle_docs_question(message)
                return

        if not self.spam_protection_enabled:
            return
        if message.author.guild_permissions.manage_messages:
            return
        if len(message.content) < 100:
            return

        is_first = await self._is_first_message(message)

        verdict = spam_filter.evaluate(
            content=message.content,
            author_created_at=message.author.created_at,
            is_first_message=is_first,
        )

        if not verdict.is_spam:
            return

        log.warning(
            "Spam detected from %s (score %d/%d): %s",
            message.author,
            verdict.score,
            verdict.threshold,
            verdict.summary,
        )

        try:
            await message.delete()
        except discord.Forbidden:
            log.error("Missing permission to delete message in %s", message.channel)
            return
        except discord.HTTPException as e:
            log.error("Failed to delete spam message: %s", e)
            return

        try:
            await message.author.send(SPAM_WARNING)
        except discord.Forbidden:
            log.info("Could not DM %s (DMs disabled)", message.author)

        await self._log_spam(message, verdict)


def main():
    try:
        bot = LeanCtxBot(use_privileged_intents=True)
        bot.run(os.environ["DISCORD_BOT_TOKEN"])
    except discord.PrivilegedIntentsRequired:
        log.warning(
            "Privileged intents not enabled in Developer Portal. "
            "Starting without spam protection. Enable MESSAGE CONTENT and "
            "SERVER MEMBERS intents at https://discord.com/developers/applications/"
        )
        bot = LeanCtxBot(use_privileged_intents=False)
        bot.run(os.environ["DISCORD_BOT_TOKEN"])


if __name__ == "__main__":
    main()
