#!/usr/bin/env bash
set -euo pipefail

REPO="${1:-/tmp/libstr}"

echo "Setting up test repo at $REPO ..."

rm -rf "$REPO"
mkdir -p "$REPO"
cd "$REPO"

git init -q
git config user.email "dev@example.com"
git config user.name "Dev"

mkdir -p src include examples thirdparty .github/workflows

# ── utf8.h generator (pure bash, no Python required) ──────────────────────
#
# Writes thirdparty/utf8.h.  Pass "1" for v1.5 (legacy decode API present)
# or "2" for v1.6 (legacy removed, fast-path decode + sanitize added).

write_utf8_h() {
    local V=$1

    if [ "$V" = "1" ]; then
        local VER="1.5.0"; local VNUM="150"; local VMINOR="5"
    else
        local VER="1.6.0"; local VNUM="160"; local VMINOR="6"
    fi

    # Signatures that change between versions
    if [ "$V" = "1" ]; then
        local SZ="int"
    else
        local SZ="size_t"
    fi

    # ── File header ────────────────────────────────────────────────────────
    cat << EOF
/*
 * utf8.h — single-header UTF-8 processing library
 *
 * Version: $VER
 * License: MIT (see bottom of file)
 *
 * Usage:
 *   Define UTF8_IMPLEMENTATION in exactly one translation unit:
 *
 *       #define UTF8_IMPLEMENTATION
 *       #include "utf8.h"
 *
 *   All other translation units include without the define.
 */

#ifndef UTF8_H
#define UTF8_H

#include <stddef.h>
#include <stdint.h>

#define UTF8_VERSION        $VNUM
#define UTF8_VERSION_MAJOR  1
#define UTF8_VERSION_MINOR  $VMINOR
#define UTF8_VERSION_PATCH  0

#ifndef UTF8_API
#  if defined(_WIN32) && defined(UTF8_SHARED)
#    ifdef UTF8_BUILDING
#      define UTF8_API __declspec(dllexport)
#    else
#      define UTF8_API __declspec(dllimport)
#    endif
#  elif defined(__GNUC__) && __GNUC__ >= 4
#    define UTF8_API __attribute__((visibility("default")))
#  else
#    define UTF8_API
#  endif
#endif

#ifdef __cplusplus
extern "C" {
#endif

/* ── Types ─────────────────────────────────────────────────────────────── */

typedef uint8_t  utf8_byte_t;
typedef uint32_t utf8_codepoint_t;

typedef struct {
    const utf8_byte_t *ptr;
    size_t             len;
} utf8_str_t;

typedef struct {
    utf8_codepoint_t codepoint;
    int              bytes_read;
    int              error;
} utf8_decode_result_t;

typedef struct {
    utf8_byte_t buf[4];
    int         len;
} utf8_encode_result_t;

/* ── Error codes ───────────────────────────────────────────────────────── */

#define UTF8_OK               0
#define UTF8_ERR_NULL        (-1)
#define UTF8_ERR_INVALID_SEQ (-2)
#define UTF8_ERR_TRUNCATED   (-3)
#define UTF8_ERR_OVERLONG    (-4)
#define UTF8_ERR_SURROGATE   (-5)
#define UTF8_ERR_OUT_OF_RANGE (-6)

#define UTF8_REPLACEMENT_CHAR 0xFFFDu
#define UTF8_MAX_CODEPOINT    0x10FFFFu

EOF

    # ── Version-specific API section ───────────────────────────────────────
    if [ "$V" = "1" ]; then
        cat << 'EOF'
/* ── Legacy decode API (deprecated since v1.5, removed in v1.6) ─────────── */
/* New code should call utf8_decode() directly.                              */

UTF8_API int utf8_decode_legacy(const utf8_byte_t *s, int len);
UTF8_API int utf8_decode_legacy_strict(const utf8_byte_t *s, int len,
                                       utf8_codepoint_t *out);
UTF8_API int utf8_next_legacy(const utf8_byte_t **s,
                              const utf8_byte_t *end);

#ifdef UTF8_IMPLEMENTATION

int utf8_decode_legacy(const utf8_byte_t *s, int len) {
    if (!s || len <= 0) return UTF8_ERR_NULL;
    utf8_byte_t b0 = s[0];
    if (b0 < 0x80) return (int)b0;
    if ((b0 & 0xe0) == 0xc0) {
        if (len < 2) return UTF8_ERR_TRUNCATED;
        if ((s[1] & 0xc0) != 0x80) return UTF8_ERR_INVALID_SEQ;
        return (int)(((b0 & 0x1f) << 6) | (s[1] & 0x3f));
    }
    if ((b0 & 0xf0) == 0xe0) {
        if (len < 3) return UTF8_ERR_TRUNCATED;
        if ((s[1] & 0xc0) != 0x80) return UTF8_ERR_INVALID_SEQ;
        if ((s[2] & 0xc0) != 0x80) return UTF8_ERR_INVALID_SEQ;
        int cp = (int)(((b0 & 0x0f) << 12) | ((s[1] & 0x3f) << 6) | (s[2] & 0x3f));
        if (cp >= 0xd800 && cp <= 0xdfff) return UTF8_ERR_SURROGATE;
        return cp;
    }
    if ((b0 & 0xf8) == 0xf0) {
        if (len < 4) return UTF8_ERR_TRUNCATED;
        if ((s[1] & 0xc0) != 0x80) return UTF8_ERR_INVALID_SEQ;
        if ((s[2] & 0xc0) != 0x80) return UTF8_ERR_INVALID_SEQ;
        if ((s[3] & 0xc0) != 0x80) return UTF8_ERR_INVALID_SEQ;
        int cp = (int)(((b0 & 0x07) << 18) | ((s[1] & 0x3f) << 12)
               | ((s[2] & 0x3f) << 6) | (s[3] & 0x3f));
        if (cp > (int)UTF8_MAX_CODEPOINT) return UTF8_ERR_OUT_OF_RANGE;
        return cp;
    }
    return UTF8_ERR_INVALID_SEQ;
}

int utf8_decode_legacy_strict(const utf8_byte_t *s, int len,
                              utf8_codepoint_t *out) {
    if (!out) return UTF8_ERR_NULL;
    int cp = utf8_decode_legacy(s, len);
    if (cp < 0) return cp;
    *out = (utf8_codepoint_t)cp;
    return 0;
}

int utf8_next_legacy(const utf8_byte_t **s, const utf8_byte_t *end) {
    if (!s || !*s || *s >= end) return UTF8_ERR_NULL;
    int cp = utf8_decode_legacy(*s, (int)(end - *s));
    if (cp < 0) { (*s)++; return cp; }
    if      (cp < 0x80)    *s += 1;
    else if (cp < 0x800)   *s += 2;
    else if (cp < 0x10000) *s += 3;
    else                   *s += 4;
    return cp;
}

#endif /* UTF8_IMPLEMENTATION */

EOF
    else
        cat << 'EOF'
/* ── Fast-path decode API (added in v1.6) ──────────────────────────────── */
/* Assumes pre-validated input. Skips bounds and error checks for speed.   */

UTF8_API utf8_codepoint_t   utf8_decode_fast(const utf8_byte_t *s);
UTF8_API int                utf8_decode_fast_n(const utf8_byte_t *s,
                                               utf8_codepoint_t *out, int n);
UTF8_API const utf8_byte_t *utf8_next_fast(const utf8_byte_t *s);
UTF8_API int                utf8_seq_len_fast(utf8_byte_t lead);

/* Returns sequence length for a given lead byte (no bounds check). */
#define UTF8_FAST_SEQ_LEN(b) (_utf8_lead_len_tab[(unsigned char)(b) >> 4])

#ifdef UTF8_IMPLEMENTATION

static const int _utf8_lead_len_tab[16] = {
    /* 0x00-0x7F */ 1, 1, 1, 1, 1, 1, 1, 1,
    /* 0x80-0xBF */ 0, 0, 0, 0,  /* continuation bytes, invalid as lead */
    /* 0xC0-0xDF */ 2, 2,
    /* 0xE0-0xEF */ 3,
    /* 0xF0-0xFF */ 4
};

utf8_codepoint_t utf8_decode_fast(const utf8_byte_t *s) {
    utf8_byte_t b = s[0];
    if (b < 0x80) return b;
    if (b < 0xe0) return ((utf8_codepoint_t)(b & 0x1f) << 6)
                       | (s[1] & 0x3f);
    if (b < 0xf0) return ((utf8_codepoint_t)(b & 0x0f) << 12)
                       | ((utf8_codepoint_t)(s[1] & 0x3f) << 6)
                       | (s[2] & 0x3f);
    return ((utf8_codepoint_t)(b & 0x07) << 18)
         | ((utf8_codepoint_t)(s[1] & 0x3f) << 12)
         | ((utf8_codepoint_t)(s[2] & 0x3f) << 6)
         | (s[3] & 0x3f);
}

int utf8_decode_fast_n(const utf8_byte_t *s, utf8_codepoint_t *out, int n) {
    const utf8_byte_t *p = s;
    for (int i = 0; i < n; i++) {
        out[i] = utf8_decode_fast(p);
        p += UTF8_FAST_SEQ_LEN(*p);
    }
    return n;
}

const utf8_byte_t *utf8_next_fast(const utf8_byte_t *s) {
    return s + UTF8_FAST_SEQ_LEN(*s);
}

int utf8_seq_len_fast(utf8_byte_t lead) {
    return UTF8_FAST_SEQ_LEN(lead);
}

#endif /* UTF8_IMPLEMENTATION */

EOF
    fi

    # ── Core decode API ────────────────────────────────────────────────────
    cat << EOF
/* ── Core decode API ───────────────────────────────────────────────────── */

UTF8_API utf8_decode_result_t utf8_decode(const utf8_byte_t *s, $SZ len);
UTF8_API int                  utf8_decode_codepoint(const utf8_byte_t *s,
                                                    $SZ len,
                                                    utf8_codepoint_t *out);
UTF8_API int                  utf8_seq_len(utf8_byte_t lead_byte);
UTF8_API int                  utf8_encode_size(utf8_codepoint_t cp);
UTF8_API const utf8_byte_t   *utf8_next(const utf8_byte_t *s,
                                         const utf8_byte_t *end);
UTF8_API const utf8_byte_t   *utf8_prev(const utf8_byte_t *s,
                                         const utf8_byte_t *start);

/* ── Encode API ─────────────────────────────────────────────────────────── */

UTF8_API utf8_encode_result_t utf8_encode(utf8_codepoint_t cp);
UTF8_API int                  utf8_encode_into(utf8_codepoint_t cp,
                                               utf8_byte_t *dst, int cap);

/* ── Validation API ─────────────────────────────────────────────────────── */

UTF8_API int    utf8_is_valid(const utf8_byte_t *s, size_t len);
UTF8_API int    utf8_count(const utf8_byte_t *s, size_t len);
UTF8_API size_t utf8_byte_offset(const utf8_byte_t *s, size_t len,
                                  size_t codepoint_idx);
EOF

    if [ "$V" = "2" ]; then
        cat << 'EOF'

/* Replaces invalid sequences with U+FFFD (added in v1.6). */
UTF8_API int    utf8_sanitize(utf8_byte_t *dst, size_t dst_cap,
                              const utf8_byte_t *src, size_t src_len);
EOF
    fi

    # ── Implementation ─────────────────────────────────────────────────────
    cat << 'EOF'

#ifdef UTF8_IMPLEMENTATION

/* Per-byte sequence-length lookup table, initialized once. */
static int _utf8_seq_lengths[256];
static int _utf8_tables_ready = 0;

static void _utf8_init_tables(void) {
    if (_utf8_tables_ready) return;
EOF

    # 256-entry table — generated with a loop to avoid hardcoding 256 lines
    for i in $(seq 0 127);   do printf "    _utf8_seq_lengths[%3d] = 1;\n" "$i"; done
    for i in $(seq 128 191); do printf "    _utf8_seq_lengths[%3d] = 0;\n" "$i"; done
    for i in $(seq 192 223); do printf "    _utf8_seq_lengths[%3d] = 2;\n" "$i"; done
    for i in $(seq 224 239); do printf "    _utf8_seq_lengths[%3d] = 3;\n" "$i"; done
    for i in $(seq 240 255); do printf "    _utf8_seq_lengths[%3d] = 4;\n" "$i"; done

    cat << EOF
    _utf8_tables_ready = 1;
}

int utf8_seq_len(utf8_byte_t lead) {
    _utf8_init_tables();
    return _utf8_seq_lengths[lead];
}

int utf8_encode_size(utf8_codepoint_t cp) {
    if (cp < 0x80)    return 1;
    if (cp < 0x800)   return 2;
    if (cp < 0x10000) return 3;
    return 4;
}

utf8_decode_result_t utf8_decode(const utf8_byte_t *s, $SZ len) {
    utf8_decode_result_t r = {0, 0, 0};
    if (!s || len == 0) { r.error = UTF8_ERR_NULL; return r; }
    utf8_byte_t b0 = s[0];
    if (b0 < 0x80) { r.codepoint = b0; r.bytes_read = 1; return r; }
    if ((b0 & 0xe0) == 0xc0) {
        if (len < 2) { r.error = UTF8_ERR_TRUNCATED; return r; }
        if ((s[1] & 0xc0) != 0x80) { r.error = UTF8_ERR_INVALID_SEQ; return r; }
        r.codepoint  = ((utf8_codepoint_t)(b0 & 0x1f) << 6) | (s[1] & 0x3f);
        r.bytes_read = 2;
        if (r.codepoint < 0x80) r.error = UTF8_ERR_OVERLONG;
        return r;
    }
    if ((b0 & 0xf0) == 0xe0) {
        if (len < 3) { r.error = UTF8_ERR_TRUNCATED; return r; }
        if ((s[1] & 0xc0) != 0x80) { r.error = UTF8_ERR_INVALID_SEQ; return r; }
        if ((s[2] & 0xc0) != 0x80) { r.error = UTF8_ERR_INVALID_SEQ; return r; }
        r.codepoint  = ((utf8_codepoint_t)(b0 & 0x0f) << 12)
                     | ((utf8_codepoint_t)(s[1] & 0x3f) << 6)
                     | (s[2] & 0x3f);
        r.bytes_read = 3;
        if (r.codepoint < 0x800) r.error = UTF8_ERR_OVERLONG;
        if (r.codepoint >= 0xd800 && r.codepoint <= 0xdfff)
            r.error = UTF8_ERR_SURROGATE;
        return r;
    }
    if ((b0 & 0xf8) == 0xf0) {
        if (len < 4) { r.error = UTF8_ERR_TRUNCATED; return r; }
        if ((s[1] & 0xc0) != 0x80) { r.error = UTF8_ERR_INVALID_SEQ; return r; }
        if ((s[2] & 0xc0) != 0x80) { r.error = UTF8_ERR_INVALID_SEQ; return r; }
        if ((s[3] & 0xc0) != 0x80) { r.error = UTF8_ERR_INVALID_SEQ; return r; }
        r.codepoint  = ((utf8_codepoint_t)(b0 & 0x07) << 18)
                     | ((utf8_codepoint_t)(s[1] & 0x3f) << 12)
                     | ((utf8_codepoint_t)(s[2] & 0x3f) << 6)
                     | (s[3] & 0x3f);
        r.bytes_read = 4;
        if (r.codepoint < 0x10000)            r.error = UTF8_ERR_OVERLONG;
        if (r.codepoint > UTF8_MAX_CODEPOINT) r.error = UTF8_ERR_OUT_OF_RANGE;
        return r;
    }
    r.error = UTF8_ERR_INVALID_SEQ;
    return r;
}

int utf8_decode_codepoint(const utf8_byte_t *s, $SZ len,
                          utf8_codepoint_t *out) {
    utf8_decode_result_t r = utf8_decode(s, len);
    if (r.error) return r.error;
    if (out) *out = r.codepoint;
    return r.bytes_read;
}

utf8_encode_result_t utf8_encode(utf8_codepoint_t cp) {
    utf8_encode_result_t r = {{0, 0, 0, 0}, 0};
    if (cp < 0x80) {
        r.buf[0] = (utf8_byte_t)cp; r.len = 1;
    } else if (cp < 0x800) {
        r.buf[0] = (utf8_byte_t)(0xc0 | (cp >> 6));
        r.buf[1] = (utf8_byte_t)(0x80 | (cp & 0x3f));
        r.len = 2;
    } else if (cp < 0x10000) {
        r.buf[0] = (utf8_byte_t)(0xe0 | (cp >> 12));
        r.buf[1] = (utf8_byte_t)(0x80 | ((cp >> 6) & 0x3f));
        r.buf[2] = (utf8_byte_t)(0x80 | (cp & 0x3f));
        r.len = 3;
    } else if (cp <= UTF8_MAX_CODEPOINT) {
        r.buf[0] = (utf8_byte_t)(0xf0 | (cp >> 18));
        r.buf[1] = (utf8_byte_t)(0x80 | ((cp >> 12) & 0x3f));
        r.buf[2] = (utf8_byte_t)(0x80 | ((cp >> 6) & 0x3f));
        r.buf[3] = (utf8_byte_t)(0x80 | (cp & 0x3f));
        r.len = 4;
    }
    return r;
}

int utf8_encode_into(utf8_codepoint_t cp, utf8_byte_t *dst, int cap) {
    utf8_encode_result_t r = utf8_encode(cp);
    if (r.len == 0 || r.len > cap) return -1;
    for (int i = 0; i < r.len; i++) dst[i] = r.buf[i];
    return r.len;
}

int utf8_is_valid(const utf8_byte_t *s, size_t len) {
    if (!s) return 0;
    size_t i = 0;
    while (i < len) {
        utf8_decode_result_t r = utf8_decode(s + i, len - i);
        if (r.error || r.bytes_read <= 0) return 0;
        i += (size_t)r.bytes_read;
    }
    return 1;
}

int utf8_count(const utf8_byte_t *s, size_t len) {
    if (!s) return 0;
    int count = 0;
    size_t i = 0;
    while (i < len) {
        utf8_decode_result_t r = utf8_decode(s + i, len - i);
        if (r.error || r.bytes_read <= 0) break;
        i += (size_t)r.bytes_read;
        count++;
    }
    return count;
}

size_t utf8_byte_offset(const utf8_byte_t *s, size_t len,
                         size_t codepoint_idx) {
    size_t i = 0, ci = 0;
    while (i < len && ci < codepoint_idx) {
        utf8_decode_result_t r = utf8_decode(s + i, len - i);
        if (r.error || r.bytes_read <= 0) break;
        i += (size_t)r.bytes_read;
        ci++;
    }
    return i;
}

const utf8_byte_t *utf8_next(const utf8_byte_t *s, const utf8_byte_t *end) {
    if (!s || s >= end) return NULL;
    utf8_decode_result_t r = utf8_decode(s, (size_t)(end - s));
    return s + (r.bytes_read > 0 ? r.bytes_read : 1);
}

const utf8_byte_t *utf8_prev(const utf8_byte_t *s, const utf8_byte_t *start) {
    if (!s || s <= start) return NULL;
    const utf8_byte_t *p = s - 1;
    while (p > start && (*p & 0xc0) == 0x80) p--;
    return p;
}
EOF

    if [ "$V" = "2" ]; then
        cat << 'EOF'

int utf8_sanitize(utf8_byte_t *dst, size_t dst_cap,
                  const utf8_byte_t *src, size_t src_len) {
    if (!dst || !src) return UTF8_ERR_NULL;
    size_t di = 0, si = 0;
    while (si < src_len && di + 4 <= dst_cap) {
        utf8_decode_result_t r = utf8_decode(src + si, src_len - si);
        if (r.error || r.bytes_read <= 0) {
            utf8_encode_into(UTF8_REPLACEMENT_CHAR, dst + di,
                             (int)(dst_cap - di));
            di += 3; si++;
        } else {
            for (int i = 0; i < r.bytes_read; i++) dst[di++] = src[si++];
        }
    }
    return (int)di;
}
EOF
    fi

    # ── Unicode BMP property tables (~49k lines total) ────────────────────
    # Three tables covering U+0000–U+FFFF, 4 entries per line.
    # Values approximate real Unicode data; exact correctness is not required
    # since this file exists to give marten a large diff to scroll through.

    cat << 'EOF'

/* ── Unicode property tables ─────────────────────────────────────────────── */
/* Cover the Basic Multilingual Plane (U+0000–U+FFFF). Values are baked in  */
/* for zero-cost lookup without external data files or runtime init.         */

/* General Category codes used below:
 *   0=Cn  1=Lu  2=Ll  3=Lo  4=Lt  5=Lm  6=Cs  7=Co
 *   8=Mn  9=Nd  10=Nl 11=No 12=Zs 13=Zl 14=Zp 15=Cc
 *   16=Cf 17=Pc 18=Pd 19=Ps 20=Pe 21=Pi 22=Pf 23=Po
 *   24=Sm 25=Sc 26=Sk 27=So */
static const uint8_t _utf8_gc_table[65536] = {
EOF
    awk -v V="$V" 'BEGIN {
        for (i = 0; i < 65536; i++) {
            if      (i < 32)                     v = 15
            else if (i == 32)                    v = 12
            else if (i < 48)                     v = 23
            else if (i < 58)                     v = 9
            else if (i < 65)                     v = 23
            else if (i < 91)                     v = 1
            else if (i < 97)                     v = 23
            else if (i < 123)                    v = 2
            else if (i < 127)                    v = 23
            else if (i == 127)                   v = 15
            else if (i < 160)                    v = 15
            else if (i == 160)                   v = 12
            else if (i < 192)                    v = 23
            else if (i < 215)                    v = 1
            else if (i == 215)                   v = 24
            else if (i < 247)                    v = 1
            else if (i == 247)                   v = 24
            else if (i < 697)                    v = (i%2==0) ? 1 : 2
            else if (i < 750)                    v = 8
            else if (i < 880)                    v = 2
            else if (i < 1024)                   v = (i%3==0) ? 1 : (i%3==1) ? 2 : 3
            else if (i < 1280)                   v = (i%2==0) ? 1 : 2
            else if (i < 1536)                   v = 2
            else if (i < 1792)                   v = 3
            else if (i < 2048)                   v = 16
            else if (i < 11904)                  v = (i%7<3) ? i%7+1 : (i%7<5) ? 8 : (i%7==5) ? 9 : 23
            else if (i < 12032)                  v = 27
            else if (i < 12256)                  v = 10
            else if (i < 12288)                  v = 23
            else if (i < 12352)                  v = 12
            else if (i < 55204)                  v = 3
            else if (i < 55296)                  v = 0
            else if (i < 57344)                  v = 6
            else if (i < 63744)                  v = 7
            else if (i < 65534)                  v = 3
            else                                 v = 0
            # v1.6: revised gc assignments for several script blocks (Unicode 16)
            if (V+0 == 2) {
                if (i >= 2048  && i < 11904) v = (i%11<4) ? i%11%3+1 : (i%11<6) ? 27 : (i%11<9) ? 8 : (i%11==9) ? 16 : 23
                if (i >= 11904 && i < 12032) v = (i%3==0) ? 27 : (i%3==1) ? 10 : 23
                if (i >= 12032 && i < 12352) v = (i%5==0) ? 10 : (i%5==1) ? 27 : (i%5==2) ? 23 : (i%5==3) ? 12 : 3
                if (i >= 12352 && i < 55204) v = (i%9==0) ? 1 : (i%9==1) ? 2 : (i%9==2) ? 10 : (i%9==3) ? 8 : (i%9<7) ? 3 : (i%9==7) ? 27 : 0
            }
            if (i%4==0) printf "    "
            printf "0x%02x", v
            if (i<65535) printf ", "
            if (i%4==3) printf "\n"
        }
    }'

    cat << 'EOF'
};

/* Bidi Category codes used below:
 *   0=L  1=R  2=AL  3=EN  4=ES  5=ET  6=AN  7=CS
 *   8=NSM  9=BN  10=B  11=S  12=WS  13=ON */
static const uint8_t _utf8_bc_table[65536] = {
EOF
    awk -v V="$V" 'BEGIN {
        for (i = 0; i < 65536; i++) {
            if      (i < 9)                      v = 9
            else if (i <= 12 && i != 10)         v = 11
            else if (i == 10 || i == 13)         v = 10
            else if (i < 28)                     v = 9
            else if (i == 28 || i == 29)         v = 10
            else if (i < 32)                     v = 11
            else if (i == 32)                    v = 12
            else if (i < 35)                     v = 13
            else if (i < 37)                     v = 5
            else if (i < 43)                     v = 13
            else if (i == 43)                    v = 4
            else if (i == 44)                    v = 13
            else if (i <= 46)                    v = 4
            else if (i == 47)                    v = 7
            else if (i < 58)                     v = 3
            else if (i == 58)                    v = 7
            else if (i < 65)                     v = 13
            else if (i < 91)                     v = 0
            else if (i < 97)                     v = 13
            else if (i < 123)                    v = 0
            else if (i < 127)                    v = 13
            else if (i == 127)                   v = 9
            else if (i < 160)                    v = 9
            else if (i == 160)                   v = 12
            else if (i < 1425)                   v = (i%3==0) ? 0 : (i%3==1) ? 13 : 5
            else if (i < 1536)                   v = 8
            else if (i < 1792)                   v = (i%3==0) ? 1 : (i%3==1) ? 2 : 8
            else if (i < 2048)                   v = (i%4==0) ? 1 : 8
            else if (i < 8192)                   v = (i%8==0) ? 0 : (i%8==1) ? 1 : (i%8==2) ? 2 : (i%8==3) ? 8 : (i%8==4) ? 9 : (i%8==5) ? 12 : 13
            else if (i < 8294)                   v = 12
            else if (i < 8352)                   v = (i%3==0) ? 5 : 13
            else if (i < 8400)                   v = 5
            else if (i < 8432)                   v = 8
            else if (i < 55296)                  v = (i%5==0) ? 0 : (i%5==1) ? 1 : (i%5==2) ? 13 : (i%5==3) ? 3 : 8
            else if (i < 57344)                  v = 9
            else                                 v = (i%6==0) ? 0 : (i%6==1) ? 1 : (i%6==2) ? 13 : (i%6==3) ? 5 : (i%6==4) ? 8 : 9
            # v1.6: revised bidi classes for CJK and supplementary blocks (Unicode 16)
            if (V+0 == 2 && i >= 8432 && i < 55296)
                v = (i%7==0) ? 0 : (i%7==1) ? 1 : (i%7==2) ? 13 : (i%7==3) ? 2 : (i%7==4) ? 8 : (i%7==5) ? 5 : 3
            if (i%4==0) printf "    "
            printf "0x%02x", v
            if (i<65535) printf ", "
            if (i%4==3) printf "\n"
        }
    }'

    cat << 'EOF'
};

/* Case Fold Delta: signed offset from an uppercase codepoint to its
 * lowercase equivalent. Zero means no case mapping for this codepoint. */
static const int16_t _utf8_cf_table[65536] = {
EOF
    awk -v V="$V" 'BEGIN {
        for (i = 0; i < 65536; i++) {
            if      (i >= 65   && i < 91)              v = 32
            else if (i >= 192  && i < 215)             v = 32
            else if (i >= 216  && i < 247)             v = 32
            else if (i >= 248  && i < 697  && i%2==0)  v = 1
            else if (i >= 1024 && i < 1072)            v = 32
            else if (i >= 1072 && i < 1120 && i%2==0)  v = 1
            else if (i >= 7680 && i < 7936 && i%2==0)  v = 1
            else if (i >= 7944 && i < 7952)            v = -8
            else if (i >= 7960 && i < 7966)            v = -8
            else if (i >= 7976 && i < 7984)            v = -8
            else if (i >= 7992 && i < 8000)            v = -8
            else if (i >= 8008 && i < 8012)            v = -8
            else if (i >= 8025 && i < 8032 && i%2==1)  v = -8
            else if (i >= 8040 && i < 8048)            v = -8
            else                                       v = 0
            # v1.6: extended case fold coverage for additional script ranges
            if (V+0 == 2) {
                if (i >= 1120  && i < 1136  && i%2==0) v = 1
                if (i >= 4256  && i < 4272  && i%2==0) v = 48
                if (i >= 7952  && i < 7960)             v = -8
                if (i >= 11264 && i < 11280 && i%2==0) v = 48
                if (i >= 24832 && i < 24848 && i%2==0) v = 1
                if (i >= 33024 && i < 33040 && i%2==0) v = 1
                if (i >= 41472 && i < 41488 && i%2==0) v = 1
                if (i >= 42560 && i < 42576 && i%2==0) v = 1
                if (i >= 65280 && i < 65296 && i%2==0) v = 32
                if (i >= 65313 && i < 65339)            v = 32
            }
            if (i%4==0) printf "    "
            printf "%d", v
            if (i<65535) printf ", "
            if (i%4==3) printf "\n"
        }
    }'

    cat << 'EOF'
};

#endif /* UTF8_IMPLEMENTATION */

#ifdef __cplusplus
}
#endif

#endif /* UTF8_H */

/*
 * MIT License
 *
 * Copyright (c) 2024 libstr contributors
 *
 * Permission is hereby granted, free of charge, to any person obtaining a
 * copy of this software and associated documentation files (the "Software"),
 * to deal in the Software without restriction, including without limitation
 * the rights to use, copy, modify, merge, publish, distribute, sublicense,
 * and/or sell copies of the Software, and to permit persons to whom the
 * Software is furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in
 * all copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS
 * OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL
 * THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 * LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING
 * FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
 * DEALINGS IN THE SOFTWARE.
 */
EOF
}

# ── Initial files ──────────────────────────────────────────────────────────

cat > include/libstr.h << 'EOF'
/* libstr - lightweight string utilities, v1.5 */
#ifndef LIBSTR_H
#define LIBSTR_H

#ifdef __cplusplus
extern "C" {
#endif

typedef struct {
    const char *ptr;
    int         len;
} str_t;

typedef struct {
    str_t value;
    int   type;
    int   offset;
} str_token_t;

int  str_parse(const char *src, int len, str_token_t *out, int max);
int  str_count_codepoints(const char *s, int len);
int  str_is_valid_utf8(const char *s, int len);
int  str_to_upper(char *dst, const char *src, int len);
int  str_to_lower(char *dst, const char *src, int len);

/* deprecated: compat API — will be removed in v2.0 */
int  str_compat_decode(const char *s, int len);
int  str_compat_encode(char *dst, int codepoint);
void str_compat_init(void);

#ifdef __cplusplus
}
#endif

#endif /* LIBSTR_H */
EOF

cat > src/compat.h << 'EOF'
#ifndef LIBSTR_COMPAT_H
#define LIBSTR_COMPAT_H

/* Legacy compatibility shim — deprecated since v1.4, removed in v2.0.
 * New code should use utf8_decode / utf8_encode directly. */

void compat_init(void);
void compat_shutdown(void);
int  compat_decode(const unsigned char *s, int len);
int  compat_encode(unsigned char *dst, unsigned int codepoint);

#endif /* LIBSTR_COMPAT_H */
EOF

cat > src/compat.c << 'EOF'
#include "compat.h"
#include <string.h>

static int _initialized = 0;

void compat_init(void) {
    _initialized = 1;
}

void compat_shutdown(void) {
    _initialized = 0;
}

int compat_decode(const unsigned char *s, int len) {
    if (!_initialized || !s || len <= 0) return -1;
    if ((s[0] & 0x80) == 0) return s[0];
    if ((s[0] & 0xe0) == 0xc0 && len >= 2)
        return ((s[0] & 0x1f) << 6) | (s[1] & 0x3f);
    if ((s[0] & 0xf0) == 0xe0 && len >= 3)
        return ((s[0] & 0x0f) << 12) | ((s[1] & 0x3f) << 6) | (s[2] & 0x3f);
    return -1;
}

int compat_encode(unsigned char *dst, unsigned int cp) {
    if (!dst) return -1;
    if (cp < 0x80)    { dst[0] = (unsigned char)cp; return 1; }
    if (cp < 0x800)   { dst[0] = 0xc0 | (cp >> 6);
                        dst[1] = 0x80 | (cp & 0x3f); return 2; }
    if (cp < 0x10000) { dst[0] = 0xe0 | (cp >> 12);
                        dst[1] = 0x80 | ((cp >> 6) & 0x3f);
                        dst[2] = 0x80 | (cp & 0x3f); return 3; }
    return -1;
}
EOF

cat > src/parser.h << 'EOF'
#ifndef LIBSTR_PARSER_H
#define LIBSTR_PARSER_H

#include "../include/libstr.h"

typedef struct {
    const char *src;
    int         pos;
    int         len;
    int         flags;
} str_parser_t;

#define STR_PARSER_FLAG_STRICT  0x01
#define STR_PARSER_FLAG_SKIP_WS 0x02

int  str_parser_init(str_parser_t *p, const char *src, int len, int flags);
int  str_parser_next(str_parser_t *p, str_token_t *tok);
int  str_parser_peek(str_parser_t *p, str_token_t *tok);
void str_parser_reset(str_parser_t *p);

#endif /* LIBSTR_PARSER_H */
EOF

cat > src/parser.c << 'EOF'
#include "parser.h"
#include "compat.h"
#include "../thirdparty/utf8.h"
#include <string.h>

int str_parser_init(str_parser_t *p, const char *src, int len, int flags) {
    if (!p || !src || len <= 0) return -1;
    compat_init();
    p->src   = src;
    p->pos   = 0;
    p->len   = len;
    p->flags = flags;
    return 0;
}

void str_parser_reset(str_parser_t *p) {
    if (!p) return;
    p->pos = 0;
}

static int skip_whitespace(str_parser_t *p) {
    while (p->pos < p->len) {
        unsigned char c = (unsigned char)p->src[p->pos];
        if (c != ' ' && c != '\t' && c != '\n' && c != '\r') break;
        p->pos++;
    }
    return p->pos;
}

int str_parser_next(str_parser_t *p, str_token_t *tok) {
    if (!p || !tok) return -1;
    if (p->flags & STR_PARSER_FLAG_SKIP_WS) skip_whitespace(p);
    if (p->pos >= p->len) return 0;

    int start = p->pos;
    int cp = utf8_decode_legacy((const unsigned char *)p->src + p->pos,
                                p->len - p->pos);
    if (cp < 0) return -1;

    int sz = utf8_encode_size((unsigned)cp);
    tok->value.ptr = p->src + start;
    tok->value.len = sz;
    tok->type      = (cp < 0x80) ? 0 : 1;
    tok->offset    = start;
    p->pos        += sz;
    return 1;
}

int str_parser_peek(str_parser_t *p, str_token_t *tok) {
    if (!p || !tok) return -1;
    int saved = p->pos;
    int r = str_parser_next(p, tok);
    p->pos = saved;
    return r;
}
EOF

cat > src/utils.h << 'EOF'
#ifndef LIBSTR_UTILS_H
#define LIBSTR_UTILS_H

int str_count_codepoints(const char *s, int len);
int str_is_valid_utf8(const char *s, int len);
int str_to_upper(char *dst, const char *src, int len);
int str_to_lower(char *dst, const char *src, int len);
int str_byte_len(const char *s, int max_codepoints);

#endif /* LIBSTR_UTILS_H */
EOF

cat > src/utils.c << 'EOF'
#include "utils.h"
#include "../thirdparty/utf8.h"
#include <string.h>

int str_count_codepoints(const char *s, int len) {
    if (!s || len <= 0) return 0;
    int count = 0, i = 0;
    while (i < len) {
        int cp = utf8_decode_legacy((const unsigned char *)s + i, len - i);
        if (cp < 0) break;
        i += utf8_encode_size((unsigned)cp);
        count++;
    }
    return count;
}

int str_is_valid_utf8(const char *s, int len) {
    if (!s) return 0;
    int i = 0;
    while (i < len) {
        int cp = utf8_decode_legacy((const unsigned char *)s + i, len - i);
        if (cp < 0) return 0;
        i += utf8_encode_size((unsigned)cp);
    }
    return 1;
}

int str_to_upper(char *dst, const char *src, int len) {
    if (!dst || !src || len <= 0) return -1;
    for (int i = 0; i < len; i++) {
        unsigned char c = (unsigned char)src[i];
        dst[i] = (char)((c >= 'a' && c <= 'z') ? c - 32 : c);
    }
    dst[len] = '\0';
    return 0;
}

int str_to_lower(char *dst, const char *src, int len) {
    if (!dst || !src || len <= 0) return -1;
    for (int i = 0; i < len; i++) {
        unsigned char c = (unsigned char)src[i];
        dst[i] = (char)((c >= 'A' && c <= 'Z') ? c + 32 : c);
    }
    dst[len] = '\0';
    return 0;
}

int str_byte_len(const char *s, int max_cp) {
    if (!s || max_cp <= 0) return 0;
    int i = 0, count = 0;
    while (count < max_cp) {
        int cp = utf8_decode_legacy((const unsigned char *)s + i, 4);
        if (cp < 0) break;
        i += utf8_encode_size((unsigned)cp);
        count++;
    }
    return i;
}
EOF

cat > examples/basic.c << 'EOF'
#include <stdio.h>
#include "../include/libstr.h"

int main(void) {
    const char *input = "hello, w\xc3\xb6rld";
    int len = 13;

    str_compat_init();

    str_token_t tokens[64];
    int n = str_parse(input, len, tokens, 64);
    printf("parsed %d tokens\n", n);

    int cp_count = str_count_codepoints(input, len);
    printf("codepoints: %d\n", cp_count);

    printf("valid utf-8: %s\n", str_is_valid_utf8(input, len) ? "yes" : "no");
    return 0;
}
EOF

cat > .github/workflows/ci.yml << 'EOF'
name: CI

on:
  push:
    branches: [main]
  pull_request:

jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Build library
        run: make

      - name: Run tests
        run: make test

      - name: Check compat link
        run: |
          cc -o /tmp/test_basic examples/basic.c libstr.a -lc++
          /tmp/test_basic

      - name: Verify static archive
        run: |
          nm libstr.a | grep -q str_parse
          echo "archive OK"
EOF

cat > Makefile << 'EOF'
CC     = cc
CFLAGS = -Wall -Wextra -std=c11 -Ithirdparty
AR     = ar

SRC    = src/parser.c src/utils.c src/compat.c
OBJ    = $(SRC:.c=.o)
TARGET = libstr.a

$(TARGET): $(OBJ)
	$(AR) rcs $@ $^

%.o: %.c
	$(CC) $(CFLAGS) -c $< -o $@

test: $(TARGET)
	$(CC) $(CFLAGS) -o /tmp/test_basic examples/basic.c $(TARGET)
	/tmp/test_basic

check-link: $(TARGET)
	cc -o /tmp/check_link examples/basic.c $(TARGET) -lc++
	/tmp/check_link

clean:
	rm -f $(OBJ) $(TARGET)
EOF

write_utf8_h 1 > thirdparty/utf8.h

# ── Initial commit ─────────────────────────────────────────────────────────
git add -A
git commit -q -m "initial: libstr v1.5 with legacy compat shim"

echo "  committed initial state"

# ── Apply changes (unstaged) ───────────────────────────────────────────────

cat > include/libstr.h << 'EOF'
/* libstr - lightweight string utilities, v1.6 */
#ifndef LIBSTR_H
#define LIBSTR_H

#ifdef __cplusplus
extern "C" {
#endif

typedef struct {
    const char *ptr;
    int         len;
} str_t;

typedef struct {
    str_t value;
    int   type;
    int   offset;
} str_token_t;

int  str_parse(const char *src, int len, str_token_t *out, int max);
int  str_count_codepoints(const char *s, int len);
int  str_is_valid_utf8(const char *s, int len);
int  str_to_upper(char *dst, const char *src, int len);
int  str_to_lower(char *dst, const char *src, int len);

#ifdef __cplusplus
}
#endif

#endif /* LIBSTR_H */
EOF

cat > src/parser.c << 'EOF'
#include "parser.h"
#include "../thirdparty/utf8.h"
#include <string.h>

int str_parser_init(str_parser_t *p, const char *src, int len, int flags) {
    if (!p || !src || len <= 0) return -1;
    p->src   = src;
    p->pos   = 0;
    p->len   = len;
    p->flags = flags;
    return 0;
}

void str_parser_reset(str_parser_t *p) {
    if (!p) return;
    p->pos = 0;
}

static int skip_whitespace(str_parser_t *p) {
    while (p->pos < p->len) {
        unsigned char c = (unsigned char)p->src[p->pos];
        if (c != ' ' && c != '\t' && c != '\n' && c != '\r') break;
        p->pos++;
    }
    return p->pos;
}

int str_parser_next(str_parser_t *p, str_token_t *tok) {
    if (!p || !tok) return -1;
    if (p->flags & STR_PARSER_FLAG_SKIP_WS) skip_whitespace(p);
    if (p->pos >= p->len) return 0;

    int start = p->pos;
    utf8_codepoint_t cp;
    int n = utf8_decode_codepoint((const utf8_byte_t *)p->src + p->pos,
                                  (size_t)(p->len - p->pos), &cp);
    if (n < 0) return -1;

    tok->value.ptr = p->src + start;
    tok->value.len = n;
    tok->type      = (cp < 0x80) ? 0 : 1;
    tok->offset    = start;
    p->pos        += n;
    return 1;
}

int str_parser_peek(str_parser_t *p, str_token_t *tok) {
    if (!p || !tok) return -1;
    int saved = p->pos;
    int r = str_parser_next(p, tok);
    p->pos = saved;
    return r;
}
EOF

cat > src/utils.c << 'EOF'
#include "utils.h"
#include "../thirdparty/utf8.h"
#include <string.h>

int str_count_codepoints(const char *s, int len) {
    if (!s || len <= 0) return 0;
    int count = 0, i = 0;
    while (i < len) {
        utf8_codepoint_t cp;
        int n = utf8_decode_codepoint((const utf8_byte_t *)s + i,
                                      (size_t)(len - i), &cp);
        if (n < 0) break;
        i += n;
        count++;
    }
    return count;
}

int str_is_valid_utf8(const char *s, int len) {
    if (!s) return 0;
    return utf8_is_valid((const utf8_byte_t *)s, (size_t)len);
}

int str_to_upper(char *dst, const char *src, int len) {
    if (!dst || !src || len <= 0) return -1;
    for (int i = 0; i < len; i++) {
        unsigned char c = (unsigned char)src[i];
        dst[i] = (char)((c >= 'a' && c <= 'z') ? c - 32 : c);
    }
    dst[len] = '\0';
    return 0;
}

int str_to_lower(char *dst, const char *src, int len) {
    if (!dst || !src || len <= 0) return -1;
    for (int i = 0; i < len; i++) {
        unsigned char c = (unsigned char)src[i];
        dst[i] = (char)((c >= 'A' && c <= 'Z') ? c + 32 : c);
    }
    dst[len] = '\0';
    return 0;
}

int str_byte_len(const char *s, int max_cp) {
    if (!s || max_cp <= 0) return 0;
    int i = 0, count = 0;
    while (count < max_cp) {
        utf8_codepoint_t cp;
        int n = utf8_decode_codepoint((const utf8_byte_t *)s + i, 4, &cp);
        if (n < 0) break;
        i += n;
        count++;
    }
    return i;
}
EOF

cat > examples/basic.c << 'EOF'
#include <stdio.h>
#include "../include/libstr.h"

int main(void) {
    const char *input = "hello, w\xc3\xb6rld";
    int len = 13;

    str_token_t tokens[64];
    int n = str_parse(input, len, tokens, 64);
    printf("parsed %d tokens\n", n);

    int cp_count = str_count_codepoints(input, len);
    printf("codepoints: %d\n", cp_count);

    printf("valid utf-8: %s\n", str_is_valid_utf8(input, len) ? "yes" : "no");
    return 0;
}
EOF

cat > .github/workflows/ci.yml << 'EOF'
name: CI

on:
  push:
    branches: [main]
  pull_request:

jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Build library
        run: make

      - name: Run tests
        run: make test

      - name: Check pure-C link
        run: |
          cc -o /tmp/test_basic examples/basic.c libstr.a
          /tmp/test_basic

      - name: Verify static archive
        run: |
          nm libstr.a | grep -q str_parse
          echo "archive OK"
EOF

cat > Makefile << 'EOF'
CC     = cc
CFLAGS = -Wall -Wextra -std=c11 -Ithirdparty
AR     = ar

SRC    = src/parser.c src/utils.c
OBJ    = $(SRC:.c=.o)
TARGET = libstr.a

$(TARGET): $(OBJ)
	$(AR) rcs $@ $^

%.o: %.c
	$(CC) $(CFLAGS) -c $< -o $@

test: $(TARGET)
	$(CC) $(CFLAGS) -o /tmp/test_basic examples/basic.c $(TARGET)
	/tmp/test_basic

check-link: $(TARGET)
	cc -o /tmp/check_link examples/basic.c $(TARGET)
	/tmp/check_link

clean:
	rm -f $(OBJ) $(TARGET)
EOF

rm src/compat.c

write_utf8_h 2 > thirdparty/utf8.h

echo "  applied working tree changes"
echo ""
echo "Done. To use:"
echo "  cd $REPO && marten"
