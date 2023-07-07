import {Profile} from "../../types/wrapper";
import {Avatar, Group, Stack, Text} from "@mantine/core";
import {MdVpnKey} from "react-icons/md";
import React from "react";

interface IParams {
    profile: Profile
}

export function UserProfileLayout({profile}: IParams) {
    return <Group noWrap={true}>
        <Avatar
            src={`https://ui-avatars.com/api/?name=${profile.name}&background=random&rounded=true`}
            radius={"md"}/>

        <Stack spacing={1}>
            <Text size="md" weight={500}>{profile.name}</Text>
            <Text size="sm">{profile.role}</Text>
        </Stack>

        {profile.has_key &&
            <MdVpnKey size="1.125rem"/>
        }
    </Group>
}
