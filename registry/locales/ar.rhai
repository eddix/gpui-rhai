fn locale() {
    #{
        locale: "ar",
        direction: "right_to_left",
        messages: #{
            "calendar.clear": "مسح التاريخ",
            "calendar.next_month": "الشهر التالي",
            "calendar.open": "فتح التقويم",
            "calendar.previous_month": "الشهر السابق",
            "common.clear": "مسح",
            "common.close": "إغلاق",
            "common.loading": "جارٍ التحميل",
            "common.no_results": "لا توجد نتائج",
            "pagination.items": "عناصر",
            "pagination.next": "الصفحة التالية",
            "pagination.page": "الصفحة",
            "pagination.per_page": "عناصر في الصفحة",
            "pagination.previous": "الصفحة السابقة",
        },
        calendar: #{
            first_weekday: "saturday",
            months: #{
                short: ["ينا", "فبر", "مار", "أبر", "ماي", "يون", "يول", "أغس", "سبت", "أكت", "نوف", "ديس"],
                long: ["يناير", "فبراير", "مارس", "أبريل", "مايو", "يونيو", "يوليو", "أغسطس", "سبتمبر", "أكتوبر", "نوفمبر", "ديسمبر"],
            },
            weekdays: #{
                short: ["أحد", "اثن", "ثلا", "أرب", "خمي", "جمع", "سبت"],
                long: ["الأحد", "الاثنين", "الثلاثاء", "الأربعاء", "الخميس", "الجمعة", "السبت"],
            },
            date_patterns: #{
                month_year: "MMMM yyyy",
                short: "d/M/yyyy",
                medium: "d MMM yyyy",
                long: "EEEE، d MMMM yyyy",
            },
        },
        number: #{
            digits: ["٠", "١", "٢", "٣", "٤", "٥", "٦", "٧", "٨", "٩"],
            decimal_separator: "٫",
            grouping_separator: "٬",
            primary_group_size: 3,
            secondary_group_size: 3,
            minus_sign: "−",
        },
    }
}
