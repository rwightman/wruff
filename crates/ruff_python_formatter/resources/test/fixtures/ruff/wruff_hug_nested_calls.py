self.pre_logits = nn.Sequential(OrderedDict([
    ("fc", nn.Linear(in_features, hidden_size, **dd)),
    ("act", act_layer()),
]))

block_args.update(dict(
    dw_kernel_size=_parse_ksize(options["k"]),
    num_heads=int(options["h"]),
    key_dim=kv_dim,
    value_dim=kv_dim,
    kv_stride=int(options.get("v", 1)),
    noskip=skip is False,
))

convs.append(layers.conv_norm_act(
    in_chs,
    out_chs,
    kernel_size=kernel_size,
    stride=stride,
    groups=groups,
    apply_act=False,
    **dd,
))
