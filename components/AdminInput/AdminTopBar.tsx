import {ActionIcon, Group, Title} from "@mantine/core";
import {MdAdd, MdRefresh} from "react-icons/md";
import React from "react";

interface IParams {
    title: string;
    onAdd: () => void;
    onRefresh: () => Promise<void>;
}

export default function AdminTopBar({title, onAdd, onRefresh}: IParams) {
    return <Group position={'apart'}>
        <Title order={2}>{title}</Title>

        <Group spacing={'lg'}>
            <ActionIcon
                variant={'filled'}
                color={'green'}
                radius={'xl'}
                size={36}
                onClick={onAdd}>
                <MdAdd size={28}/>
            </ActionIcon>

            <ActionIcon variant={'filled'} radius={'xl'} size={36} onClick={onRefresh}>
                <MdRefresh size={28}/>
            </ActionIcon>
        </Group>
    </Group>
}
