def configure_formatter_behavior(
    first_argument, second_argument, third_argument, fourth_argument
):
    return helper_call(
        first_argument, second_argument, third_argument, fourth_argument
    )


result = helper_call(first_argument, second_argument, third_argument, fourth_argument)

compact = sequence[start + 1 : stop + 1]
stepped = sequence[offset :: step + 1]
nested = sequence[start + delta : stop - 1]
attribute = sequence[record.value : stop]
negated_attribute = sequence[-args.n :]
scaled = sequence[index * 2 : stop * 2]
floored = sequence[index // 2 : stop // 2]
