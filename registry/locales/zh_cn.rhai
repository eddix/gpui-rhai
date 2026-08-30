fn locale() {
    #{
        locale: "zh-CN",
        direction: "left_to_right",
        messages: #{
            "calendar.clear": "清除日期",
            "calendar.next_month": "下个月",
            "calendar.open": "打开日历",
            "calendar.previous_month": "上个月",
            "common.clear": "清除",
            "common.close": "关闭",
            "common.loading": "加载中",
            "common.no_results": "没有结果",
            "pagination.items": "条",
            "pagination.next": "下一页",
            "pagination.page": "第",
            "pagination.per_page": "每页条数",
            "pagination.previous": "上一页",
        },
        calendar: #{
            first_weekday: "monday",
            months: #{
                short: ["1月", "2月", "3月", "4月", "5月", "6月", "7月", "8月", "9月", "10月", "11月", "12月"],
                long: ["一月", "二月", "三月", "四月", "五月", "六月", "七月", "八月", "九月", "十月", "十一月", "十二月"],
            },
            weekdays: #{
                short: ["周日", "周一", "周二", "周三", "周四", "周五", "周六"],
                long: ["星期日", "星期一", "星期二", "星期三", "星期四", "星期五", "星期六"],
            },
            date_patterns: #{
                month_year: "yyyy年MMMM",
                short: "yyyy/M/d",
                medium: "yyyy年M月d日",
                long: "yyyy年M月d日 EEEE",
            },
        },
        number: #{
            digits: ["0", "1", "2", "3", "4", "5", "6", "7", "8", "9"],
            decimal_separator: ".",
            grouping_separator: ",",
            primary_group_size: 3,
            secondary_group_size: 3,
            minus_sign: "-",
        },
    }
}
