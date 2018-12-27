#include <stdbool.h>			// Standard C library needed for bool
#include <stdint.h>				// Standard C library needed for uint8_t, uint32_t etc
#include <stdarg.h>				// Standard C library needed for varadic arguments

/***************************************************************************}
{		 		    PRIVATE VARIABLE DEFINITIONS				            }
****************************************************************************/

#include "usb-dependency.h"

#ifndef EOF
#define EOF (-1)
#endif

/* Number of bits in an 'unsigned long'.  */
#define LONG_BITS (8 * sizeof(unsigned long))

static void ulong_to_string(unsigned long num, char *str,
                            unsigned int base, bool alt_digits);

enum integer_size {
    SHORT_SHORT_SIZE,
    SHORT_SIZE,
    REGULAR_SIZE,
    LONG_SIZE
};

int emb_strlen(char const *s) {
    int ret=0;
    for(;*s;++s)
        ++ret;
    return ret;
}

/**
 * @ingroup libxc
 *
 * Write formatted output.
 *
 * This is a simplified implementation, and not all standard conversion
 * specifications are supported.  A conversion specification (a sequence
 * beginning with the @c '%' character) is divided into 5 parts, the first four
 * of which are optional.  The following list explains the features supported by
 * this implementation, broken down by part of the conversion specification:
 *
 * 1. Flags:
 *    0+ of the following:
 *     - @c "-" to specify left-justification
 *     - @c "0" to specify zero padding
 *     - @c "#" to specify special prefix, hex = 0x octal = 0, ignored otherwise
 *
 * 2. Minimum field width:
 *    0-1 of the following:
 *      - A series of decimal digits not beginning with 0 that specifies the
 *        minimum field width as a non-negative integer
 *      - @c "*", indicating that the minimum field width is given as an @c int
 *        variadic argument
 *
 * 3. Precision:
 *    0-1 of the following:
 *      - @c ".PREC", where @c PREC is a series of decimal digits that specifies
 *        the precision as a non-negative integer
 *      - @c "*", indicating that the precision is given as an @c int variadic
 *         argument
 *
 * 4. Length modifier for signed and unsigned integer conversions:
 *    0-1 of the following:
 *      - @c "hh"   for <code>signed char</code>  or <code>unsigned char</code>
 *      - @c "h"    for <code>signed short</code> or <code>unsigned short</code>
 *      - @c "l"    for <code>signed long</code>  or <code>unsigned long</code>
 *
 * 5. Conversion specifier character:
 *    1 of the following:
 *      - @c "\%d"  to format a  signed integer   in decimal        (base 10)
 *      - @c "\%i"  to format a  signed integer   in decimal        (base 10)
 *      - @c "\%b"  to format an unsigned integer in binary         (base 2)
 *      - @c "\%o"  to format an unsigned integer in octal          (base 8)
 *      - @c "\%u"  to format an unsigned integer in decimal        (base 10)
 *      - @c "\%x"  to format an unsigned integer in lower case hex (base 16)
 *      - @c "\%X"  to format an unsigned integer in upper case hex (base 16)
 *      - @c "\%p"  to format a pointer in upper case hex			(base 16)
 *      - @c "\%c"  to format a single character
 *      - @c "\%s"  to format a null-terminated string, or "(null)" for a @c NULL pointer
 *      - @c "\%\%" to format a literal percent sign
 *
 * If a feature is not mentioned above, assume it is not supported.
 *
 * @param fmt
 *      Format string.
 * @param ap
 *      Variable-length list of values that will be formatted.
 * @param putc_func
 *      Character output function.  It is passed two arguments; the first will
 *      be the character to output, and the second will be @p putc_arg.  It is
 *      expected to return @c EOF on failure.
 * @param putc_arg
 *      Second argument to @p putc_func.
 *
 * @return
 *      number of characters written on success, or @c EOF on failure
 */
int _doprnt(const char *fmt, va_list ap, int (*putc_func) (int, void*), void* putc_arg)
{
    int chars_written = 0;      /* Number of characters written so far  */

    int i;
    char *str;                  /* Pointer to characters to output      */
    char string[LONG_BITS + 1]; /* Buffer for numeric conversions       */

    bool leftjust;              /* true = left-justified,
                                   false = right-justified              */
    unsigned char hashtype;     /* 0 = no hash flag, 1 = hashflag set   */
                                /* 2 = octal 0 prefix 3 = 0x hex prefix */
    char pad_char;              /* Padding character                    */
    char prefix[2];				/* Prefix characters                    */
    int fmin;                   /* Minimum field width                  */
    int prec;                   /* Field precision                      */
    enum integer_size size;     /* Length modifier                      */
    char sign;                  /* Set to '-' for negative decimals     */

    long larg;                  /* Numeric argument                     */
    unsigned long ularg;        /* Numeric argument                     */
    bool alt_digits;            /* Use alternate digits?                */
    unsigned int base;          /* Base to use for printing.            */

    int prefix_len;				/* No of characters in prefix string    */
    int len_str;                /* No. of chars from str to output      */
    int num_zeroes;             /* No. of zeroes to precede number with
                                   (for precision, not zero padding)    */
    int len_nonpadding;         /* Total No. of non-padding chars to
                                   output                               */
    int len_padding;            /* No. of padding chars to output       */

    const char *spec_start;     /* Start of this format specifier.      */

    while (*fmt != '\0')
    {
        if (*fmt == '%' && *++fmt != '%')
        {
            /* Parsing a conversion specification ---
             * Consists of 5 parts, as noted in comments below  */

            spec_start = fmt - 1;

            /*************************************
             * 1. Zero or more flags             *
             *************************************/
            prefix_len = 0;			/* Default: No prefix string */
            pad_char = ' ';			/* Default: space padding    */
            leftjust = false;		/* Default: right-justified  */
            hashtype = 0;			/* Default: no hash flag     */
            for ( ; ; fmt++)
            {
                /* Switch on next potential flag character  */
                switch (*fmt)
                {
                case '-':
                    /* '-' flag: left-justified conversion  */
                    leftjust = true;
                    break;
                case '#':
                    /* '#' flag: alternative conversion     */
                    hashtype = 1;
                    break;
                case '0':
                    /* '0' flag: pad field width with zeroes
                     * (valid for numeric conversions only)  */
                    pad_char = '0';
                    break;

                default:
                    /* Not a flag character; continue on.  */
                    goto flags_scanned;
                }
            }

    flags_scanned:

            /*************************************
             * 2. Optional minimum field width   *
             *************************************/
            fmin = 0;
            if (*fmt == '*')
            {
                fmin = va_arg(ap, int);
                if (fmin < 0)
                {
                    /* C99 7.19.6.1:  A negative field width argument is taken
                     * as a '-' flag followed by a positive field width.  */
                    fmin = -fmin;
                    leftjust = true;
                }
                fmt++;
            }
            else
            {
                while ('0' <= *fmt && *fmt <= '9')
                {
                    fmin *= 10;
                    fmin += (*fmt - '0');
                    fmt++;
                }
            }

            /* C99 7.19.6.1:  If both the '0' and '-' flags appear, the '0'
             * flag is ignored.  */
            if (leftjust)
            {
                pad_char = ' ';
            }

            /*************************************
             * 3. Optional precision             *
             *************************************/
            prec = -1;
            if (*fmt == '.')
            {
                fmt++;
                if (*fmt == '*')
                {
                    prec = va_arg(ap, int);
                    fmt++;
                    /* C99 7.19.6.1:  A negative precision argument is taken as
                     * if the precision were omitted.  */
                }
                else
                {
                    prec = 0;
                    while ('0' <= *fmt && *fmt <= '9')
                    {
                        prec *= 10;
                        prec += (*fmt - '0');
                        fmt++;
                    }
                    /* C99 7.19.6.1:  If only the period is specified, the
                     * precision is taken as zero.  */
                }
            }

            /*************************************
             * 4. Optional length modifier       *
             *************************************/

            size = REGULAR_SIZE;
            if (*fmt == 'l')
            {
                size = LONG_SIZE;
                fmt++;
            }
            else if (*fmt == 'h')
            {
                fmt++;
                if (*fmt == 'h')
                {
                    size = SHORT_SHORT_SIZE;
                    fmt++;
                }
                else
                {
                    size = SHORT_SIZE;
                }
            }

            /*************************************
             * 5. Conversion specifier character *
             *************************************/

            /* Set defaults  */
            base = 0;              /* Not numeric          */
            sign = '\0';           /* No sign              */
            str = string;          /* Use temporary space  */
            alt_digits = false;    /* Use normal digits    */

            /* Switch on the format specifier character.  */
            switch (*fmt)
            {
            case 'c':
                /* Format a character.  */
                /* Note: 'char' is promoted to 'int' when passed as a variadic
                 * argument.  */
                string[0] = (unsigned char)va_arg(ap, int);
                string[1] = '\0';
                break;

            case 's':
                /* Format a string.  */
                str = va_arg(ap, char *);
                if (str == NULL)
                {
                    str = "(null)";
                }
                break;

            case 'i':
            case 'd':
                /* Format a signed integer in base 10  */
                base = 10;
                if (size == LONG_SIZE)
                {
                    larg = va_arg(ap, long);
                }
                else
                {
                    /* Note: 'signed char' and 'short' are promoted to 'int'
                     * when passed as variadic arguments.  */
                    larg = va_arg(ap, int);
                }
                ularg = larg;
                if (larg < 0)
                {
                    sign = '-';
                    ularg = -ularg;
                    /* Note: negating the argument while still in signed form
                     * would produce undefined behavior in the case of the most
                     * negative value.  */
                }
                break;

            case 'u':
                /* Format an unsigned integer in base 10  */
                base = 10;
                goto handle_unsigned;

            case 'o':
                /* Format an unsigned integer in base 8  */
                base = 8;
                /* Hashflag set on octal display means put 0 at front   */
                if (hashtype == 1) { hashtype = 2; };
                goto handle_unsigned;

            case 'X':
                /* Format an unsigned integer in base 16 (upper case)  */
                alt_digits = true;
                /* case X drops into case x ... only alt_digits diff */
            case 'x':
                /* Format an unsigned integer in base 16 (lower case)  */
                base = 16;
                /* Hashflag set on hex display means put 0x at front   */
                if (hashtype == 1) { hashtype = 3; };
                goto handle_unsigned;

            case 'p':
                /* Format an unsigned integer in base 16 (lower case)  */
                base = 16;
                /* Hashflag set on hex display means put 0x at front   */
                hashtype = 3;
                ularg = (unsigned long) va_arg(ap, void*);
                pad_char = '0';
                fmin = sizeof(void*);
                break;

            case 'b':
                /* Format an unsigned integer in base 2  */
                base = 2;
                goto handle_unsigned;

            handle_unsigned:
                if (size == LONG_SIZE)
                {
                    ularg = va_arg(ap, unsigned long);
                }
                else
                {
                    /* Note: 'unsigned char' and 'unsigned short' are promoted
                     * to 'unsigned int' when passed as variadic arguments.  */
                    ularg = va_arg(ap, unsigned int);
                }
                break;

            default:
                /* Unknown format specifier; this also includes the case where
                 * we encounted the end of the format string prematurely.  Write
                 * the '%' literally and continue parsing from the next
                 * character.  */
                fmt = spec_start;
                goto literal;
            }

            /* Advance past format specifier character.  */
            fmt++;

            /* If an integer conversion, convert the absolute value of the
             * number to a string in the temporary buffer.  */
            if (base != 0)
            {

                /* If hash type octal and it is not zero */
                if ( hashtype == 2 && ularg != 0 )
                { 
                    /* add a 0 to front as prefix */
                    prefix[0] = '0';
                    prefix_len = 1;
                }

                /* If hash type hex  */
                if (hashtype == 3)
                {
                    /* add a x0 to front as prefix */
                    prefix[0] = '0';
                    prefix[1] = 'x';
                    prefix_len = 2;
                }

                /* run conversion avoiding prefix that may have been added */
                ulong_to_string(ularg, &str[0], base, alt_digits);

            }

            /* Do length computations.  */

            num_zeroes = 0;
            len_str = emb_strlen(str);
            if (prec >= 0)
            {
                /* Precision specified.  */
                if (base == 0)
                {
                    /* String conversions:  Precision specifies *maximum* number
                     * of string characters to output.  */
                    if (prec < len_str)
                    {
                        len_str = prec;
                    }
                }
                else
                {
                    /* Integer conversions:  Precision specifies *minimum*
                     * number of integer digits to output.  */
                    if (prec > len_str)
                    {
                        num_zeroes = prec - len_str;
                    }
                    /* C99 7.19.6.1:  For integer conversions, if a precision is
                     * specified, the '0' flag is ignored.  */
                    pad_char = ' ';
                }
            }

            /* Calculate length of everything except the field padding.  */
            len_nonpadding = len_str + num_zeroes + (sign != '\0');

            /* Calculate number of padding characters to use.  */
            len_padding = 0;
            if (len_nonpadding < fmin)
            {
                len_padding = fmin - len_nonpadding;
            }

            /* As a shortcut (especially with regards to sign handling), if the
             * output is right-justified with zero padding, treat the padding
             * zeroes in the same way as leading zeroes generated from integer
             * precision specifications.  */
            if (!leftjust && pad_char == '0')
            {
                num_zeroes = len_padding;
                len_padding = 0;
            }

            /* If we have a prefix string output it  */
            for (int i = 0; i < prefix_len; i++)
            {
                if ((*putc_func) (prefix[i], putc_arg) == EOF)
                {
                    return EOF;
                }
                chars_written++;
            }


            /* If right-justified, pad on left.  */
            if (!leftjust)
            {
                for (i = 0; i < len_padding; i++)
                {
                    if ((*putc_func) (pad_char, putc_arg) == EOF)
                    {
                        return EOF;
                    }
                    chars_written++;
                }
            }

            /* Output sign if needed.  */
            if (sign != '\0')
            {
                if ((*putc_func) (sign, putc_arg) == EOF)
                {
                    return EOF;
                }
                chars_written++;
            }

            /* Output any zeroes needed because of precision specified in
             * integer conversions.  */
            for (i = 0; i < num_zeroes; i++)
            {
                if ((*putc_func) ('0', putc_arg) == EOF)
                {
                    return EOF;
                }
                chars_written++;
            }

            /* Output any needed characters from str.  */
            for (i = 0; i < len_str; i++)
            {
                if ((*putc_func) (str[i], putc_arg) == EOF)
                {
                    return EOF;
                }
                chars_written++;
            }

            /* If left-justified, pad on right.  */
            if (leftjust)
            {
                for (i = 0; i < len_padding; i++)
                {
                    if ((*putc_func) (pad_char, putc_arg) == EOF)
                    {
                        return EOF;
                    }
                    chars_written++;
                }
            }
        }
        else
        {
literal:
            /* Literal character.  */
            if ((*putc_func) (*fmt, putc_arg) == EOF)
            {
                return EOF;
            }
            chars_written++;
            fmt++;
        }
    }
    return chars_written;
}

static const char digits_lc[16] = "0123456789abcdef";
static const char digits_uc[16] = "0123456789ABCDEF";
static const unsigned char base_to_nbits[17] = {
    [2]  = 1,
    [4]  = 2,
    [8]  = 3,
    [16] = 4,
};

/* When this is not defined, special code is used to speed up
 * converting numbers to a a string with power-of-two base.  */
/*#define ALWAYS_USE_DIVISION*/

/**
 * Convert an unsigned long integer to a string.
 *
 * @param num
 *      Number to convert.
 * @param str
 *      Buffer into which to write the string.
 * @param base
 *      Base to use; must be less than or equal to 16.
 * @param alt_digits
 *      TRUE if hex digits should be upper case rather than lowercase.
 */
static void ulong_to_string(unsigned long num, char *str,
                            unsigned int base, bool alt_digits)
{
    const char *digits = digits_lc;
    char temp[LONG_BITS + 1];
    int i;

    /* Print the string to a temporary buffer in
     * reverse order before copying it to @str.  */
    digits = (alt_digits) ? digits_uc : digits_lc;
    temp[0] = '\0';
    i = 1;
#if ALWAYS_USE_DIVISION
    if (TRUE)
#else
    if (base_to_nbits[base] == 0)
#endif
    {
        /* Use modulo operation and integral division.  */
        for (;;)
        {
            temp[i] = digits[num % base];
            num /= base;
            if (num == 0)
            {
                break;
            }
            i++;
        }
    }
    else
    {
        /* Use masking and shifting (works when base is a power of 2) */
        unsigned char shift = base_to_nbits[base];
        unsigned long mask = (1UL << shift) - 1;
        for (;;)
        {
            temp[i] = digits[num & mask];
            num >>= shift;
            if (num == 0)
            {
                break;
            }
            i++;
        }
    }

    /* Reverse string and copy it to @str.  */
    do
    {
        *str++ = temp[i--];
    } while (i >= 0);
}

/*
* Routine called by _doprnt() to output each character.
*/
 static int prn_to_buf (int c, void* buf)
{
    char **sptr = (char **)buf;
    char *s = *sptr;

    *s++ = c;
    *sptr = s;
    return (int)c;
}

/*++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++}
{				 	PUBLIC FORMATTED OUTPUT ROUTINES					    }
{++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++*/

/*-[ sprintf ]--------------------------------------------------------------}
. Writes the C string formatted by fmt to the given buffer, replacing any
. format specifier in the same way as printf.
.
. DEPRECATED:
. Using sprintf, there is no way to limit the number of characters written,
. which means the function is susceptible to buffer overruns. The suggested
. replacement is snprintf.
.
. RETURN:
.	SUCCESS: Positive number of characters written to the provided buffer.
.	   FAIL: -1
. 19Oct17 LdB
.--------------------------------------------------------------------------*/
int emb_printf (const char* fmt, ...)
{
	char buf[120];
    va_list ap;
    char *s;

    s = buf;
    va_start(ap, fmt);
    _doprnt(fmt, ap, prn_to_buf, (void*)&s);
    va_end(ap);
    *s = '\0';

    rustos_print(buf);

    return s - buf;
}
