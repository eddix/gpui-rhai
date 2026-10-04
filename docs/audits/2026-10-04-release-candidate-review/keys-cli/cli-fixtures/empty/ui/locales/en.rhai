fn locale() {
    #{
        locale: "en",
        direction: "left_to_right",
        messages: #{
            "calendar.clear": "Clear date",
            "calendar.next_month": "Next month",
            "calendar.open": "Open calendar",
            "calendar.previous_month": "Previous month",
            "common.clear": "Clear",
            "common.close": "Close",
            "common.loading": "Loading",
            "common.no_results": "No results",
            "pagination.items": "items",
            "pagination.next": "Next page",
            "pagination.page": "Page",
            "pagination.per_page": "Items per page",
            "pagination.previous": "Previous page",
        },
        calendar: #{
            first_weekday: "sunday",
            months: #{
                short: ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"],
                long: ["January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December"],
            },
            weekdays: #{
                short: ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"],
                long: ["Sunday", "Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday"],
            },
            date_patterns: #{
                month_year: "MMMM yyyy",
                short: "MM/dd/yyyy",
                medium: "MMM d, yyyy",
                long: "EEEE, MMMM d, yyyy",
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
